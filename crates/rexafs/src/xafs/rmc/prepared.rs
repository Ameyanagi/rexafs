//! Typed ReFEFF GENFMT adapter. ReFEFF's public core/I/O APIs own the scattering
//! physics; this module avoids repeated file handoffs after electronic setup.
use super::*;
use ::refeff::{core, io, ArtifactSelection};
use core::{GenfmtDriverSetup, TransitionBMatrix};
use ndarray::{Array1, Array2, Array3};
use num_complex::Complex64;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn backend(e: impl std::fmt::Display) -> RmcError {
    RmcError::Calculator(e.to_string())
}

/// ReFEFF amplitude and phase for one explicit path, with degeneracy and S₀²
/// equal to one and no additional Debye–Waller factor. Added after 0.2.9
/// (unreleased). Amplitude already includes reference-path propagation losses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathScattering {
    /// Source wave numbers, in Å⁻¹, strictly increasing.
    pub k: Vec<f64>,
    /// Dimensionless amplitude on the source grid.
    pub amplitude: Vec<f64>,
    /// Unwrapped phase in radians, excluding `2*k*half_length`.
    pub phase: Vec<f64>,
    /// Reference half-path length in Å.
    pub half_length: f64,
}
impl PathScattering {
    /// Evaluate the ReFEFF path on a requested increasing k grid in Å⁻¹. Uses
    /// ReFEFF's cubic `terp1` amplitude/phase interpolation onto its 0.05 Å⁻¹
    /// output grid, then the RMC calculator's linear resampling. This matches
    /// ordinary EXAFS FF2X without S₀², energy corrections or extra damping.
    /// Typed values avoid the pipeline's text/PAD rounding; expect small rounding
    /// differences, not bitwise equality. Extrapolation is rejected.
    pub fn sample(&self, k: &[f64]) -> Result<Vec<f64>, RmcError> {
        self.sample_length(k, self.half_length)
    }
    pub(super) fn sample_length(&self, k: &[f64], length: f64) -> Result<Vec<f64>, RmcError> {
        self.table()?.sample(k, length)
    }
    pub(super) fn table(&self) -> Result<PathTable, RmcError> {
        require(
            self.k.len() >= 2
                && self.k.len() == self.amplitude.len()
                && self.k.len() == self.phase.len()
                && self.k.windows(2).all(|v| v[1] > v[0])
                && self
                    .k
                    .iter()
                    .chain(&self.amplitude)
                    .chain(&self.phase)
                    .all(|v| v.is_finite())
                && self.half_length.is_finite()
                && self.half_length > 0.,
            "invalid path scattering table",
        )?;
        let start = (self.k[0] / 0.05).trunc() as i64 + i64::from(self.k[0] > 0.);
        let count =
            ((self.k.last().unwrap() - start as f64 * 0.05) / 0.05 + 1e-3).floor() as usize + 1;
        require(count <= 10000, "path output grid exceeds 10000 points")?;
        let grid: Vec<f64> = (0..count)
            .map(|i| (start + i as i64) as f64 * 0.05)
            .collect();
        let amplitude = grid
            .iter()
            .map(|&q| core::terp1(&self.k, &self.amplitude, q).map_err(backend))
            .collect::<Result<Vec<_>, _>>()?;
        let phase = grid
            .iter()
            .map(|&q| core::terp1(&self.k, &self.phase, q).map_err(backend))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PathTable {
            grid,
            amplitude,
            phase,
        })
    }
}

pub(super) struct PathTable {
    pub grid: Vec<f64>,
    pub amplitude: Vec<f64>,
    pub phase: Vec<f64>,
}
impl PathTable {
    pub fn sample_group(
        &self,
        k: &[f64],
        lengths: &[f64],
        settings: &MomentSettings,
    ) -> Result<Vec<f64>, RmcError> {
        let expansion = PathMomentExpansion::new(lengths, settings.clone())?;
        let chi = self
            .grid
            .iter()
            .zip(&self.amplitude)
            .zip(&self.phase)
            .map(|((&q, &a), &p)| {
                Ok(a * (expansion.evaluate(q)?.sum * Complex64::from_polar(1., p)).im)
            })
            .collect::<Result<Vec<_>, RmcError>>()?;
        super::refeff::interpolate(&self.grid, &chi, k)
    }
    pub fn sample(&self, k: &[f64], length: f64) -> Result<Vec<f64>, RmcError> {
        require(
            k.len() >= 2
                && k.iter().all(|v| v.is_finite())
                && k.windows(2).all(|v| v[1] > v[0])
                && length.is_finite()
                && length > 0.,
            "invalid path sampling grid or length",
        )?;
        let chi: Vec<_> = self
            .grid
            .iter()
            .zip(&self.amplitude)
            .zip(&self.phase)
            .map(|((&q, &a), &p)| a * (2. * q * length + p).sin())
            .collect();
        super::refeff::interpolate(&self.grid, &chi, k)
    }
}

/// Immutable electronic/scattering context prepared at one explicit reference
/// absorber. Potentials, phase tensors, radial factors and transition tables are
/// held in memory. Each subsequent path uses ReFEFF's public GENFMT kernels;
/// it does not rerun atomic/POT/XSPH/PATH or use temporary files. This is a
/// pinned-electronic-state approximation, separate from any shared-path basis.
/// Ordinary EXAFS and linear polarization are supported, with S₀²=1 and no
/// extra disorder damping. See `doc/rmc-acceleration.md` (unreleased).
pub struct PreparedRefeffContext {
    reference: Configuration,
    absorber: usize,
    options: RefeffOptions,
    genfmt: io::GenfmtInput,
    global: io::GlobalInput,
    phase: io::PhaseBinData,
    tables: io::PhaseBinGenfmtData,
    setup: GenfmtDriverSetup,
    bmatrix: TransitionBMatrix,
    angular_momenta: Array1<i32>,
    radial: Array3<Complex64>,
    legendre: Array2<f64>,
    edge_start: usize,
    potentials: BTreeMap<u8, usize>,
    identity: String,
    stats: RefeffCacheStats,
}
impl PreparedRefeffContext {
    /// Run electronic setup once. Copies the reference and settings; failed
    /// setup returns an error. FEFF's path amplitude criteria are deliberately
    /// disabled for explicit-path evaluation, so weak paths are not lost through
    /// trial-dependent screening. The caller owns catalogue truncation limits.
    pub fn prepare(
        reference: Configuration,
        absorber: usize,
        edge: Edge,
        options: RefeffOptions,
    ) -> Result<Self, RmcError> {
        Self::prepare_controlled(
            reference,
            absorber,
            edge,
            options,
            ::refeff::CancellationToken::default(),
        )
    }
    pub(super) fn prepare_controlled(
        reference: Configuration,
        absorber: usize,
        edge: Edge,
        options: RefeffOptions,
        cancellation: ::refeff::CancellationToken,
    ) -> Result<Self, RmcError> {
        let mut calculator = RefeffCalculator::new(options.clone())?;
        calculator.set_cancellation(cancellation);
        let input = calculator
            .input_for(&reference, absorber, edge)?
            .replace("CONTROL 1 1 1 1 1 1", "CONTROL 1 1 1 0 0 0");
        let selected = ["phase.bin", "global.inp", "genfmt.inp", "ff2x.inp"];
        let result = calculator.run_pipeline(
            input,
            &options,
            ArtifactSelection::Paths(selected.iter().map(std::path::PathBuf::from).collect()),
            None,
        )?;
        let text = |name: &str| -> Result<&str, RmcError> {
            std::str::from_utf8(
                result
                    .artifacts
                    .get(name)
                    .ok_or_else(|| backend(format!("prepared context missing {name}")))?,
            )
            .map_err(backend)
        };
        let phase = io::phase_bin::parse_phase_bin(text("phase.bin")?).map_err(backend)?;
        let global =
            io::GlobalInput::parse_str("global.inp", text("global.inp")?).map_err(backend)?;
        let mut genfmt =
            io::GenfmtInput::parse_str("genfmt.inp", text("genfmt.inp")?).map_err(backend)?;
        let ff2x = io::Ff2xInput::parse_str("ff2x.inp", text("ff2x.inp")?).map_err(backend)?;
        require(
            global.control.do_nrixs == 0
                && ff2x.control.mbconv == 0
                && ff2x.corrections.vrcorr == 0.
                && ff2x.corrections.vicorr == 0.
                && ff2x.corrections.s02 == 1.
                && ff2x.debye.sig2g == 0.
                && ff2x.debye.tk <= 1e-3,
            "prepared context supports ordinary EXAFS without extra corrections/damping",
        )?;
        genfmt.control.critcw = 0.;
        let tables = phase.to_genfmt_data().map_err(backend)?;
        let setup = io::genfmt_driver_setup_from_handoffs("refeff-rust", &genfmt, &global, &phase)
            .map_err(backend)?;
        let bmatrix = io::genfmt_ordinary_transition_b_matrix_from_handoffs(&global, &phase)
            .map_err(backend)?;
        let angular_momenta = Array1::from_vec(bmatrix.orbital_momenta.to_vec());
        let radial = io::genfmt_ordinary_spin_radial_factors_from_phase(&phase).map_err(backend)?;
        let legendre = io::genfmt_core_legendre_normalization_from_feff_dims().map_err(backend)?;
        let edge_start = io::genfmt_edge_start_index_from_phase(&phase).map_err(backend)?;
        let potentials = tables
            .atomic_numbers
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, &z)| (z as u8, i))
            .collect();
        let mut digest = Sha256::new();
        digest.update(text("phase.bin")?);
        digest.update(text("global.inp")?);
        digest.update(text("genfmt.inp")?);
        let hash: String = digest
            .finalize()
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect();
        let identity = format!("refeff-0.4.0/typed-path-v1/{hash}");
        let mut stats = calculator.stats();
        stats.full_calculations = 1;
        Ok(Self {
            reference,
            absorber,
            options,
            genfmt,
            global,
            phase,
            tables,
            setup,
            bmatrix,
            angular_momenta,
            radial,
            legendre,
            edge_start,
            potentials,
            identity,
            stats,
        })
    }
    /// Fixed phase/polarization identity for safe sharing of numerical path tables.
    pub fn identity(&self) -> &str {
        &self.identity
    }
    /// Work and stage timings for the single electronic setup.
    pub fn setup_stats(&self) -> &RefeffCacheStats {
        &self.stats
    }
    /// Borrow the fixed electronic reference geometry.
    pub fn reference(&self) -> &Configuration {
        &self.reference
    }
    /// Borrow the options used for electronic setup.
    pub fn options(&self) -> &RefeffOptions {
        &self.options
    }
    /// Evaluate an explicit path's scattering table using current geometry and
    /// the prepared phase shifts. This operation is thread-safe and independent
    /// of call order. Species/order/cell and central absorber must match setup.
    /// No path-radius or displacement screening is applied here; catalogues enforce it.
    pub fn scattering(
        &self,
        path: &ScatteringPath,
        c: &Configuration,
    ) -> Result<PathScattering, RmcError> {
        require(
            path.absorber == self.absorber
                && c.cell == self.reference.cell
                && c.atoms.len() == self.reference.atoms.len()
                && c.atoms
                    .iter()
                    .zip(&self.reference.atoms)
                    .all(|(a, b)| a.atomic_number == b.atomic_number),
            "prepared path changed electronic reference topology/absorber",
        )?;
        require(
            (1..=5).contains(&path.scatterers.len()),
            "prepared path requires 2..=6 legs",
        )?;
        let positions = path.positions(c)?;
        let origin = positions[0];
        let mut potentials = Vec::new();
        for vertex in &path.scatterers {
            potentials.push(if vertex.atom == self.absorber && vertex.image == [0; 3] {
                0
            } else {
                *self
                    .potentials
                    .get(&c.atoms[vertex.atom].atomic_number)
                    .ok_or_else(|| backend("path species absent from prepared potentials"))?
            });
        }
        potentials.push(0);
        let positions_bohr = Array2::from_shape_vec(
            (potentials.len(), 3),
            positions
                .iter()
                .skip(1)
                .flat_map(|p| (0..3).map(move |i| (p[i] - origin[i]) / core::FEFF_BOHR_ANGSTROM))
                .collect(),
        )
        .map_err(backend)?;
        require(
            positions_bohr.iter().all(|v| v.is_finite()),
            "nonfinite explicit path coordinates",
        )?;
        let mut path_data = io::PathsDatGenfmtPath {
            index: 1,
            degeneracy: 1.,
            effective_half_path_length_bohr: path.half_length(c)? / core::FEFF_BOHR_ANGSTROM,
            potential_indices: Array1::from_vec(potentials),
            positions_bohr,
        };
        self.canonicalize(&mut path_data)?;
        self.scattering_data(path_data)
    }
    // ReFEFF's path search chooses one canonical time direction before GENFMT.
    // The two raw directions differ numerically in the backend; evaluating both
    // would disagree with its canonical-direction/degeneracy convention.
    fn canonicalize(&self, path: &mut io::PathsDatGenfmtPath) -> Result<(), RmcError> {
        let count = path.potential_indices.len();
        let mut positions = Array2::<f64>::zeros((count, 3));
        let mut potentials = vec![0usize];
        for i in 0..count - 1 {
            for axis in 0..3 {
                positions[(i + 1, axis)] =
                    path.positions_bohr[(i, axis)] * core::FEFF_BOHR_ANGSTROM;
            }
            potentials.push(path.potential_indices[i]);
        }
        let indices: Vec<_> = (0..count - 1)
            .map(|i| {
                if path.potential_indices[i] == 0 {
                    0
                } else {
                    i + 1
                }
            })
            .collect();
        let canonical =
            core::path_canonical_representation(core::PathCanonicalRepresentationInput {
                atom_positions: positions.view(),
                path_indices: &indices,
                atom_potentials: &potentials,
                polarization: self.global.control.ipol,
                spin: if self.phase.spin_count > 1 {
                    self.global.control.ispin.abs()
                } else {
                    self.global.control.ispin
                },
                electric_vector: self.global.evec,
                incident_vector: self.global.xivec,
                symmetry_case_override: None,
                force_no_symmetry: false,
            })
            .map_err(backend)?;
        if canonical.reversed {
            for i in 0..(count - 1) / 2 {
                let j = count - 2 - i;
                path.potential_indices.swap(i, j);
                for axis in 0..3 {
                    path.positions_bohr.swap((i, axis), (j, axis));
                }
            }
        }
        Ok(())
    }
    fn scattering_data(
        &self,
        path_data: io::PathsDatGenfmtPath,
    ) -> Result<PathScattering, RmcError> {
        let paths = [path_data];
        let setups = io::genfmt_ordinary_path_setups_from_handoffs(
            &self.genfmt,
            &self.global,
            &self.phase,
            &paths,
        )
        .map_err(backend)?;
        let transitions = io::genfmt_ordinary_transition_matrices_from_handoff_setups(
            &self.global,
            &self.phase,
            &setups,
            &self.bmatrix,
        )
        .map_err(backend)?;
        let output = core::genfmt_ordinary_path_evaluation_from_driver_setup(
            core::GenfmtOrdinaryPathEvaluationFromDriverSetupInput {
                energy_grid: core::GenfmtOrdinaryPathEnergyGridFromDriverSetupInput {
                    driver_setup: &self.setup,
                    path_setup: &setups[0],
                    path_potential_indices: paths[0].potential_indices.view(),
                    angular_limits: self.tables.angular_limits.view(),
                    spin_phase_shifts: self.tables.spin_phase_shifts.view(),
                    signed_angular_offset: self.tables.signed_angular_offset,
                    momentum_zero_epsilon: 1e-16,
                    xnlm: self.legendre.view(),
                    transition_angular_momenta: self.angular_momenta.view(),
                    spin_radial_factors: self.radial.view(),
                    transition_matrices: transitions[0].matrices.view(),
                    transition_magnetic_offset: (transitions[0].matrices.shape()[1] - 1) / 2,
                },
                path_index: 1,
                print_level: 0,
                curved_wave_criterion_percent: 0.,
                edge_start_index: self.edge_start,
                active_energy_count: self.phase.main_energy_count,
                degeneracy: 1.,
                current_normalization: -1.,
                positions: paths[0].positions_bohr.view(),
                phase_epsilon: 1e-16,
            },
        )
        .map_err(backend)?;
        let retained = output
            .finalization
            .output_decision
            .retained_output
            .ok_or_else(|| backend("unscreened ReFEFF path was not retained"))?;
        let grid = &self.setup.header.wave_numbers;
        let n = grid
            .windows(2)
            .into_iter()
            .position(|w| w[1] <= w[0])
            .map_or(grid.len(), |i| i + 1);
        require(n >= 2, "prepared phase has no increasing EXAFS grid")?;
        let mut phase = retained.phases.to_vec();
        for i in 1..phase.len() {
            phase[i] = core::remove_phase_jump(phase[i], phase[i - 1]).map_err(backend)?;
        }
        Ok(PathScattering {
            k: grid
                .iter()
                .take(n)
                .map(|v| v / core::FEFF_BOHR_ANGSTROM)
                .collect(),
            amplitude: retained.amplitudes.iter().take(n).copied().collect(),
            phase: phase[..n].to_vec(),
            half_length: retained.effective_half_path_length_bohr * core::FEFF_BOHR_ANGSTROM,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_cuprite_path_records_match_pipeline_without_file_roundtrips() {
        let library = crate::structure::BuiltinLibrary::get().unwrap();
        let c =
            Configuration::from_structure(&library.structure("cu2o_cuprite").unwrap(), [2, 2, 2])
                .unwrap();
        let absorber = c.atoms.iter().position(|a| a.atomic_number == 29).unwrap();
        let options = RefeffOptions {
            path_radius: 4.5,
            path_criteria: [0., 0.],
            ..Default::default()
        };
        let context =
            PreparedRefeffContext::prepare(c.clone(), absorber, Edge::K, options.clone()).unwrap();
        let mut calc = RefeffCalculator::new(options.clone()).unwrap();
        let input = calc.input_for(&c, absorber, Edge::K).unwrap();
        let output = calc
            .run_pipeline(input, &options, ArtifactSelection::All, None)
            .unwrap();
        let native = output.spectra.chi.as_ref().unwrap();
        let k: Vec<_> = (0..191).map(|i| 2.5 + i as f64 * 0.05).collect();
        let reference = super::super::refeff::interpolate(
            &native.wave_number.to_vec(),
            &native.chi.to_vec(),
            &k,
        )
        .unwrap();
        let records = output.paths.as_ref().unwrap();
        let mut chi = vec![0.; k.len()];
        for p in &records.paths {
            let table = context
                .scattering_data(
                    p.to_genfmt_path(context.phase.potential_count() - 1)
                        .unwrap(),
                )
                .unwrap();
            for (v, x) in chi.iter_mut().zip(table.sample(&k).unwrap()) {
                *v += x * p.degeneracy;
            }
        }
        let relative = (chi
            .iter()
            .zip(&reference)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            / reference.iter().map(|x| x * x).sum::<f64>())
        .sqrt();
        println!(
            "Cuprite record count {}, degeneracy {}, typed record error {relative}",
            records.paths.len(),
            records.paths.iter().map(|p| p.degeneracy).sum::<f64>()
        );
        assert!(
            relative < 5e-5,
            "typed GENFMT/FF2X assembly error {relative}"
        );
        fn key(p: &io::PathsDatGenfmtPath) -> Vec<i64> {
            let mut potentials = p.potential_indices.to_vec();
            potentials.pop();
            let positions: Vec<[f64; 3]> = std::iter::once([0.; 3])
                .chain(
                    p.positions_bohr
                        .rows()
                        .into_iter()
                        .take(potentials.len())
                        .map(|r| [r[0], r[1], r[2]]),
                )
                .collect();
            let forward = |reverse: bool| {
                let mut ids: Vec<_> = (1..positions.len()).collect();
                if reverse {
                    ids.reverse();
                }
                ids.insert(0, 0);
                let mut key = vec![potentials.len() as i64];
                key.extend(ids.iter().skip(1).map(|&i| potentials[i - 1] as i64));
                for i in 0..ids.len() {
                    for j in i + 1..ids.len() {
                        key.push(
                            (super::super::geometry::distance(positions[ids[i]], positions[ids[j]])
                                * 1e4)
                                .round() as i64,
                        );
                    }
                }
                key
            };
            forward(false).min(forward(true))
        }
        let mut expected = std::collections::BTreeMap::new();
        for p in &records.paths {
            let data = p
                .to_genfmt_path(context.phase.potential_count() - 1)
                .unwrap();
            let values = context
                .scattering_data(data.clone())
                .unwrap()
                .sample(&k)
                .unwrap();
            expected.insert(
                key(&data),
                (p.index, p.degeneracy, values, 0usize, vec![0.; k.len()]),
            );
        }
        let cat = PathCatalogue::new(
            c.clone(),
            absorber,
            PathCatalogueSettings {
                radius: 4.5,
                max_legs: 4,
                displacement: 0.,
                ..Default::default()
            },
        )
        .unwrap();
        let mut concrete_sum = vec![0.; k.len()];
        for p in cat.paths() {
            let coords = p.positions(&c).unwrap();
            let data = io::PathsDatGenfmtPath {
                index: 1,
                degeneracy: 1.,
                effective_half_path_length_bohr: p.half_length(&c).unwrap()
                    / core::FEFF_BOHR_ANGSTROM,
                potential_indices: Array1::from_iter(
                    p.scatterers
                        .iter()
                        .map(|v| {
                            if v.atom == absorber && v.image == [0; 3] {
                                0
                            } else {
                                context.potentials[&c.atoms[v.atom].atomic_number]
                            }
                        })
                        .chain(std::iter::once(0)),
                ),
                positions_bohr: Array2::from_shape_vec(
                    (p.scatterers.len() + 1, 3),
                    coords
                        .iter()
                        .skip(1)
                        .flat_map(|r| {
                            (0..3).map(|i| (r[i] - coords[0][i]) / core::FEFF_BOHR_ANGSTROM)
                        })
                        .collect(),
                )
                .unwrap(),
            };
            let group = expected
                .get_mut(&key(&data))
                .expect("catalogue path family absent from ReFEFF records");
            group.3 += 1;
            for (v, x) in group
                .4
                .iter_mut()
                .zip(context.scattering(p, &c).unwrap().sample(&k).unwrap())
            {
                *v += x;
            }
        }
        for (_, (index, deg, representative, count, sum)) in expected {
            assert_eq!(deg, count as f64, "path family {index}");
            let error = (sum
                .iter()
                .zip(&representative)
                .map(|(a, b)| (a - b * deg).powi(2))
                .sum::<f64>()
                / representative
                    .iter()
                    .map(|v| (v * deg).powi(2))
                    .sum::<f64>())
            .sqrt();
            assert!(error < 5e-4, "path family {index} mismatch {error}");
            for (v, x) in concrete_sum.iter_mut().zip(sum) {
                *v += x;
            }
        }
        let concrete_error = (concrete_sum
            .iter()
            .zip(&reference)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            / reference.iter().map(|v| v * v).sum::<f64>())
        .sqrt();
        assert!(
            concrete_error < 2e-4,
            "concrete catalogue sum mismatch {concrete_error}"
        );
    }
}
