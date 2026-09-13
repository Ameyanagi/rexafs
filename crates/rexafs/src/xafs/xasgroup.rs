#![allow(dead_code)]
#![allow(unused_imports)]

#[cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
// Standard library dependencies
use std::error::Error;
use std::fmt;
use std::mem;

// External dependencies
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

// load dependencies
use super::errors::DataError;
use super::tools::{merge_spectra, MergeConfig};
use super::xasspectrum;
use super::XAFSError;

use itertools::Itertools;

// Load local traits
use crate::xafs::io::xasdatatype::XASGroupFile;
use crate::xafs::io::{xafs_bson::XASBson, xafs_json::XASJson};
use crate::xafs::xasspectrum::XASSpectrum;

/// One failed stage operation, associated with its original zero-based group index.
#[derive(Debug, Clone)]
pub struct BatchSpectrumError {
    /// Position in the group when the batch operation started.
    pub index: usize,
    /// Typed error returned by that spectrum.
    pub source: XAFSError,
}

/// Collected failures after every spectrum has been attempted.
/// Successful spectra retain their results; this is not a transactional rollback.
#[derive(Debug, Clone)]
pub struct BatchProcessError {
    /// Failures sorted by original spectrum index for both execution modes.
    pub errors: Vec<BatchSpectrumError>,
}

impl fmt::Display for BatchProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "batch processing failed for {} spectrum(s)",
            self.errors.len()
        )?;
        for err in &self.errors {
            write!(f, "; index {}: {}", err.index, err.source)?;
        }
        Ok(())
    }
}

impl Error for BatchProcessError {}

/// Ordered, owned collection of spectra, also exported as [`crate::Group`].
///
/// Stage methods process every member using that spectrum's settings. The default
/// methods use Rayon parallel iteration; `_seq` variants run sequentially. Empty
/// groups succeed without work. On failure, all errors are collected in index order
/// and successful members remain processed. No spectrum is merged automatically.
///
/// Legacy `get_spectrum` methods clamp oversized indices to the final spectrum.
/// Use `group.spectra.get(index)` for ordinary checked indexing instead.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct XASGroup {
    /// Spectra in display/processing order; direct edits follow [`XASSpectrum`] invalidation rules.
    pub spectra: Vec<XASSpectrum>,
}

impl Default for XASGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl XASGroup {
    /// Create an empty collection with no allocated spectrum data.
    pub fn new() -> Self {
        Self {
            spectra: Vec::new(),
        }
    }

    /// Return the number of spectra.
    pub fn len(&self) -> usize {
        self.spectra.len()
    }

    /// Return whether the collection contains no spectra.
    pub fn is_empty(&self) -> bool {
        self.spectra.is_empty()
    }

    /// Move one spectrum into the end of the collection without cloning its buffers.
    pub fn add_spectrum(&mut self, spectrum: XASSpectrum) -> &mut Self {
        self.spectra.push(spectrum);
        self
    }

    /// Move spectra into the end of the collection, preserving their order.
    pub fn add_spectra(&mut self, spectra: Vec<XASSpectrum>) -> &mut Self {
        self.spectra.extend(spectra);
        self
    }

    /// Move all members of another collection into the end of this collection.
    pub fn add_group(&mut self, group: XASGroup) -> &mut Self {
        self.spectra.extend(group.spectra);
        self
    }

    /// Remove one member by zero-based index. An out-of-range index returns an error
    /// and leaves the collection unchanged.
    pub fn remove_spectrum(&mut self, index: usize) -> Result<&mut Self, XAFSError> {
        if index >= self.spectra.len() {
            return Err(DataError::IndexOutOfRange {
                index,
                length: self.spectra.len(),
            }
            .into());
        }

        self.spectra.remove(index);
        Ok(self)
    }

    /// Remove members at the given original zero-based indices. Duplicate indices are
    /// removed once; out-of-range indices are ignored. Remaining order is preserved.
    pub fn remove_spectra(&mut self, indices: &[usize]) -> Result<&mut Self, XAFSError> {
        if self.spectra.is_empty() || indices.is_empty() {
            return Ok(self);
        }

        let mut remove_mask = vec![false; self.len()];
        for &index in indices {
            if index < self.spectra.len() {
                remove_mask[index] = true;
            }
        }

        let mut current_index = 0usize;
        self.spectra.retain(|_| {
            let keep = !remove_mask[current_index];
            current_index += 1;
            keep
        });
        Ok(self)
    }

    /// Move one member to a position immediately before the original `to` index.
    /// `to == len()` appends; larger destinations are clamped to `len()`. An oversized
    /// source selects the final member.
    ///
    /// # Panics
    /// Panics for an empty collection; check [`Self::is_empty`] first.
    pub fn move_spectrum(&mut self, from: usize, to: usize) -> &mut Self {
        // TODO: check if it is fast enough

        let from_index = if from < self.spectra.len() {
            from
        } else {
            self.spectra.len() - 1
        };

        let to_index = if to <= self.spectra.len() {
            to
        } else {
            self.spectra.len()
        };

        if from_index + 1 == to_index {
            return self;
        }

        let tmp_spectrum = mem::take(&mut self.spectra[from_index]);
        self.spectra.insert(to_index, tmp_spectrum);

        if from_index > to_index {
            self.spectra.remove(from_index + 1);
        } else {
            self.spectra.remove(from_index);
        }

        self
    }

    /// Move selected members before the original `to` position, or append if it is
    /// beyond the end. Source indices are sorted and deduplicated; out-of-range sources
    /// are ignored, preserving the relative order of selected and remaining members.
    pub fn move_spectra(&mut self, from: &[usize], to: usize) -> &mut Self {
        let to_index = if to <= self.spectra.len() {
            to
        } else {
            self.spectra.len()
        };

        // Remove the duplicate index from the from list
        let mut from_index: Vec<usize> = from
            .as_ref()
            .iter()
            .filter(|&index| *index < self.spectra.len())
            .copied()
            .collect::<Vec<usize>>();

        from_index.sort();
        from_index.dedup();

        // Create a temporary vector to store the spectra to be moved
        // It is moved by mem::take() to avoid cloning
        let mut tmp_spectra = Vec::with_capacity(from_index.len());

        for index in from_index.iter() {
            tmp_spectra.push(mem::take(&mut self.spectra[*index]));
        }

        // Create a iterator to remove the spectra from the group
        let mut remove_mask = vec![false; self.len()];
        for index in from_index.iter().copied() {
            remove_mask[index] = true;
        }

        // Calculate the shift of the insert index
        let insert_index_shift = from_index.iter().filter(|&index| *index < to_index).count();

        let insert_index = to_index - insert_index_shift;

        let mut current_index = 0usize;
        self.spectra.retain(|_| {
            let keep = !remove_mask[current_index];
            current_index += 1;
            keep
        });

        let (left_spectra, right_spectra) = self.spectra.split_at_mut(insert_index);

        // I think this part is not very efficient
        // TODO: check if it is fast enough
        self.spectra = left_spectra
            .iter_mut()
            .chain(tmp_spectra.iter_mut())
            .chain(right_spectra.iter_mut())
            .map(mem::take)
            .collect::<Vec<XASSpectrum>>();
        self
    }

    /// Borrow a member, clamping an oversized index to the final member.
    /// Returns `EmptyGroup` when empty. Use `spectra.get(index)` to reject oversized indices.
    pub fn get_spectrum(&self, index: usize) -> Result<&XASSpectrum, XAFSError> {
        if self.spectra.is_empty() {
            return Err(DataError::EmptyGroup.into());
        }

        if index >= self.spectra.len() {
            return self
                .spectra
                .last()
                .ok_or_else(|| DataError::EmptyGroup.into());
        }

        Ok(&self.spectra[index])
    }

    /// Mutably borrow a member, clamping an oversized index to the final member.
    /// Returns `EmptyGroup` when empty. Use setters on the spectrum to invalidate cached
    /// results, or call `invalidate_derived()` after direct data/settings edits.
    pub fn get_spectrum_mut(&mut self, index: usize) -> Result<&mut XASSpectrum, XAFSError> {
        if self.spectra.is_empty() {
            return Err(DataError::EmptyGroup.into());
        }

        if index >= self.spectra.len() {
            return self
                .spectra
                .last_mut()
                .ok_or_else(|| DataError::EmptyGroup.into());
        }

        Ok(&mut self.spectra[index])
    }

    /// Merge the spectra at `indices` into a new spectrum (see
    /// [`merge_spectra`]). The first index is the master whose grid and
    /// stage configurations are used by default. The merged spectrum is
    /// returned and *not* added to the group.
    pub fn merge(&self, indices: &[usize], cfg: &MergeConfig) -> Result<XASSpectrum, XAFSError> {
        if self.spectra.is_empty() {
            return Err(DataError::EmptyGroup.into());
        }
        let members = indices
            .iter()
            .map(|&index| {
                self.spectra
                    .get(index)
                    .ok_or(XAFSError::Data(DataError::IndexOutOfRange {
                        index,
                        length: self.spectra.len(),
                    }))
            })
            .collect::<Result<Vec<_>, _>>()?;
        merge_spectra(&members, cfg)
    }

    fn collect_seq_errors<F>(&mut self, mut op: F) -> Result<&mut Self, BatchProcessError>
    where
        F: FnMut(&mut XASSpectrum) -> Result<&mut XASSpectrum, XAFSError>,
    {
        if self.spectra.is_empty() {
            return Ok(self);
        }

        let mut errors: Option<Vec<BatchSpectrumError>> = None;
        for (index, spectrum) in self.spectra.iter_mut().enumerate() {
            if let Err(source) = op(spectrum) {
                errors
                    .get_or_insert_with(|| Vec::with_capacity(4))
                    .push(BatchSpectrumError { index, source });
            }
        }

        if let Some(errors) = errors {
            Err(BatchProcessError { errors })
        } else {
            Ok(self)
        }
    }

    fn collect_par_errors<F>(&mut self, op: F) -> Result<&mut Self, BatchProcessError>
    where
        F: Fn(&mut XASSpectrum) -> Result<&mut XASSpectrum, XAFSError> + Sync + Send,
    {
        if self.spectra.is_empty() {
            return Ok(self);
        }

        let mut errors = self
            .spectra
            .par_iter_mut()
            .enumerate()
            .filter_map(|(index, spectrum)| {
                op(spectrum)
                    .err()
                    .map(|source| BatchSpectrumError { index, source })
            })
            .collect::<Vec<_>>();
        if errors.is_empty() {
            Ok(self)
        } else {
            if errors.len() > 1 {
                errors.sort_by_key(|err| err.index);
            }
            Err(BatchProcessError { errors })
        }
    }

    /// Attempt to estimate edge energies for every spectrum in parallel (the default).
    /// Uses [`XASSpectrum::find_e0`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn find_e0(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.find_e0_par()
    }

    /// Attempt to estimate edge energies for every spectrum sequentially.
    /// Uses [`XASSpectrum::find_e0`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn find_e0_seq(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.collect_seq_errors(|spectrum| spectrum.find_e0())
    }

    /// Attempt to estimate edge energies for every spectrum in parallel using Rayon.
    /// Uses [`XASSpectrum::find_e0`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn find_e0_par(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.collect_par_errors(|spectrum| spectrum.find_e0())
    }

    /// Attempt to normalize absorption for every spectrum in parallel (the default).
    /// Uses [`XASSpectrum::normalize`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn normalize(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.normalize_par()
    }

    /// Attempt to normalize absorption for every spectrum sequentially.
    /// Uses [`XASSpectrum::normalize`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn normalize_seq(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.collect_seq_errors(|spectrum| spectrum.normalize())
    }

    /// Attempt to normalize absorption for every spectrum in parallel using Rayon.
    /// Uses [`XASSpectrum::normalize`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn normalize_par(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.collect_par_errors(|spectrum| spectrum.normalize())
    }

    /// Attempt to calculate backgrounds for every spectrum in parallel (the default).
    /// Uses [`XASSpectrum::calc_background`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn calc_background(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.calc_background_par()
    }

    /// Attempt to calculate backgrounds for every spectrum sequentially.
    /// Uses [`XASSpectrum::calc_background`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn calc_background_seq(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.collect_seq_errors(|spectrum| spectrum.calc_background())
    }

    /// Attempt to calculate backgrounds for every spectrum in parallel using Rayon.
    /// Uses [`XASSpectrum::calc_background`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn calc_background_par(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.collect_par_errors(|spectrum| spectrum.calc_background())
    }

    /// Attempt to calculate forward Fourier transforms for every spectrum in parallel (the default).
    /// Uses [`XASSpectrum::fft`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn fft(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.fft_par()
    }

    /// Attempt to calculate forward Fourier transforms for every spectrum sequentially.
    /// Uses [`XASSpectrum::fft`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn fft_seq(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.collect_seq_errors(|spectrum| spectrum.fft())
    }

    /// Attempt to calculate forward Fourier transforms for every spectrum in parallel using Rayon.
    /// Uses [`XASSpectrum::fft`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn fft_par(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.collect_par_errors(|spectrum| spectrum.fft())
    }

    /// Attempt to calculate inverse Fourier transforms for every spectrum in parallel (the default).
    /// Uses [`XASSpectrum::ifft`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn ifft(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.ifft_par()
    }

    /// Attempt to calculate inverse Fourier transforms for every spectrum sequentially.
    /// Uses [`XASSpectrum::ifft`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn ifft_seq(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.collect_seq_errors(|spectrum| spectrum.ifft())
    }

    /// Attempt to calculate inverse Fourier transforms for every spectrum in parallel using Rayon.
    /// Uses [`XASSpectrum::ifft`], including its prerequisite calculations and
    /// invalidation behavior. All failures are collected; successful results are retained.
    pub fn ifft_par(&mut self) -> Result<&mut Self, BatchProcessError> {
        self.collect_par_errors(|spectrum| spectrum.ifft())
    }

    /// Replace this collection with a successfully decoded legacy BSON group file.
    /// Read/decoding failures leave this collection unchanged. This format is separate
    /// from the desktop `.rxs` project format.
    pub fn read_bson(&mut self, filename: &str) -> Result<&mut Self, XAFSError> {
        let mut xas_group_file = XASGroupFile::new();

        xas_group_file.read_bson(filename)?;

        _ = mem::replace(self, xas_group_file.data);

        Ok(self)
    }

    /// Clone this collection into a legacy group envelope and write BSON, overwriting
    /// the destination. Returns file/serialization errors. This is not the desktop
    /// project writer and provides no atomic replacement or backup guarantee.
    pub fn write_bson(&self, filename: &str) -> Result<&Self, XAFSError> {
        let mut xas_group_file = XASGroupFile::new();

        xas_group_file.name = filename.to_string();
        xas_group_file.data = self.clone();
        xas_group_file.write_bson(filename)?;

        Ok(self)
    }

    /// Read a legacy BSON group file and append all of its members without cloning.
    /// Despite the singular name, the file can contain multiple spectra. Read/decoding
    /// failures leave this collection unchanged.
    pub fn add_spectrum_from_bson(&mut self, filename: &str) -> Result<&mut Self, XAFSError> {
        let mut xas_group_file = XASGroupFile::new();
        xas_group_file.read_bson(filename)?;
        self.add_group(xas_group_file.data);

        Ok(self)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::xafs::io;
    use approx::{assert_abs_diff_eq, assert_relative_eq};

    use data_reader::reader::{load_txt_f64, Delimiter, ReaderParams};

    use crate::xafs::tests::PARAM_LOADTXT;
    use crate::xafs::tests::TEST_TOL;
    use crate::xafs::tests::TOP_DIR;

    fn assert_slice_close(left: &[f64], right: &[f64], epsilon: f64) {
        assert_eq!(left.len(), right.len());
        for (l, r) in left.iter().zip(right.iter()) {
            assert_abs_diff_eq!(l, r, epsilon = epsilon);
        }
    }

    #[test]
    fn test_xasgroup() {
        let group = XASGroup::new();

        assert_eq!(group.len(), 0);
    }

    #[test]
    fn test_add_spectrum() {
        let mut group = XASGroup::new();
        let spectrum = XASSpectrum::new();
        group.add_spectrum(spectrum.clone());
        assert_eq!(group.len(), 1);
    }

    #[test]
    fn test_remove_spectrum() {
        let mut group = XASGroup::new();
        let spectrum = XASSpectrum::new();
        group.add_spectrum(spectrum.clone());
        group.remove_spectrum(0).unwrap();
        assert_eq!(group.len(), 0);
    }

    #[test]
    fn test_move_spectrum() {
        let mut group = XASGroup::new();
        let spectrum = XASSpectrum::new();
        group.add_spectrum(spectrum.clone().set_name("spectrum1").to_owned());
        group.add_spectrum(spectrum.clone().set_name("spectrum2").to_owned());
        group.add_spectrum(spectrum.clone().set_name("spectrum3").to_owned());
        group.move_spectrum(1, 0);
        assert_eq!(group.spectra[0].name.as_ref().unwrap(), "spectrum2");

        group.move_spectrum(0, group.len());
        assert_eq!(group.spectra[2].name.as_ref().unwrap(), "spectrum2");

        group.move_spectrum(10, group.len());
        println!("{:?}", group);
        assert_eq!(group.spectra[2].name.as_ref().unwrap(), "spectrum2");

        group.move_spectrum(10, 0);
        assert_eq!(group.spectra[0].name.as_ref().unwrap(), "spectrum2");

        group.move_spectrum(0, 10);
        assert_eq!(group.spectra[2].name.as_ref().unwrap(), "spectrum2");
    }

    #[test]
    fn test_move_spectra() {
        let mut group = XASGroup::new();
        let spectrum = XASSpectrum::new();
        group.add_spectrum(spectrum.clone().set_name("spectrum1").to_owned());
        group.add_spectrum(spectrum.clone().set_name("spectrum2").to_owned());
        group.add_spectrum(spectrum.clone().set_name("spectrum3").to_owned());
        group.move_spectra(&[0, 1], 3);
        assert_eq!(group.spectra[2].name.as_ref().unwrap(), "spectrum2");
    }

    #[test]
    fn test_batch_find_e0_returns_structured_error_seq_and_par() {
        let path = String::from(TOP_DIR) + "/tests/testfiles/Ru_QAS.dat";
        let valid = io::load_spectrum_QAS_trans(&path).unwrap();
        let invalid = XASSpectrum::new();

        let mut seq_group = XASGroup::new();
        seq_group.add_spectrum(valid.clone());
        seq_group.add_spectrum(invalid.clone());
        let seq_err = seq_group.find_e0_seq().unwrap_err();
        assert_eq!(seq_err.errors.len(), 1);
        assert_eq!(seq_err.errors[0].index, 1);

        let mut par_group = XASGroup::new();
        par_group.add_spectrum(valid);
        par_group.add_spectrum(invalid);
        let par_err = par_group.find_e0_par().unwrap_err();
        assert_eq!(par_err.errors.len(), 1);
        assert_eq!(par_err.errors[0].index, 1);
    }

    #[test]
    fn test_batch_find_e0_par_multiple_errors_are_sorted_by_index() {
        let path = String::from(TOP_DIR) + "/tests/testfiles/Ru_QAS.dat";
        let valid = io::load_spectrum_QAS_trans(&path).unwrap();
        let invalid = XASSpectrum::new();

        let mut par_group = XASGroup::new();
        par_group
            .add_spectrum(invalid.clone())
            .add_spectrum(valid)
            .add_spectrum(invalid);

        let par_err = par_group.find_e0_par().unwrap_err();
        let indices = par_err
            .errors
            .iter()
            .map(|err| err.index)
            .collect::<Vec<_>>();
        assert_eq!(indices, vec![0, 2]);
    }

    #[test]
    fn test_default_and_par_error_semantics_match() {
        let path = String::from(TOP_DIR) + "/tests/testfiles/Ru_QAS.dat";
        let valid = io::load_spectrum_QAS_trans(&path).unwrap();
        let invalid = XASSpectrum::new();

        let mut par_group = XASGroup::new();
        par_group
            .add_spectrum(valid.clone())
            .add_spectrum(invalid.clone());
        let mut default_group = XASGroup::new();
        default_group.add_spectrum(valid).add_spectrum(invalid);

        let par_err = par_group.find_e0_par().unwrap_err();
        let default_err = default_group.find_e0().unwrap_err();

        assert_eq!(par_err.errors.len(), default_err.errors.len());
        for (par, default_) in par_err.errors.iter().zip(default_err.errors.iter()) {
            assert_eq!(par.index, default_.index);
            assert_eq!(par.source.to_string(), default_.source.to_string());
        }
    }

    #[test]
    fn test_seq_par_default_numerical_equivalence() {
        let path = String::from(TOP_DIR) + "/tests/testfiles/Ru_QAS.dat";
        let base = io::load_spectrum_QAS_trans(&path).unwrap();

        let mut group_seq = XASGroup::new();
        group_seq
            .add_spectrum(base.clone())
            .add_spectrum(base.clone());

        let mut group_par = group_seq.clone();
        let mut group_default = group_seq.clone();

        group_seq.find_e0_seq().unwrap();
        group_seq.normalize_seq().unwrap();
        group_seq.calc_background_seq().unwrap();
        group_seq.fft_seq().unwrap();

        group_par.find_e0_par().unwrap();
        group_par.normalize_par().unwrap();
        group_par.calc_background_par().unwrap();
        group_par.fft_par().unwrap();

        group_default.find_e0().unwrap();
        group_default.normalize().unwrap();
        group_default.calc_background().unwrap();
        group_default.fft().unwrap();

        for index in 0..group_seq.len() {
            let seq = &group_seq.spectra[index];
            let par = &group_par.spectra[index];
            let default = &group_default.spectra[index];

            assert_abs_diff_eq!(seq.e0().unwrap(), par.e0().unwrap(), epsilon = 1.0e-8);
            assert_abs_diff_eq!(par.e0().unwrap(), default.e0().unwrap(), epsilon = 1.0e-8);

            let seq_norm = seq
                .normalization
                .as_ref()
                .and_then(|method| method.get_norm())
                .unwrap();
            let par_norm = par
                .normalization
                .as_ref()
                .and_then(|method| method.get_norm())
                .unwrap();
            let default_norm = default
                .normalization
                .as_ref()
                .and_then(|method| method.get_norm())
                .unwrap();
            let seq_norm_vec = seq_norm.iter().copied().collect::<Vec<_>>();
            let par_norm_vec = par_norm.iter().copied().collect::<Vec<_>>();
            let default_norm_vec = default_norm.iter().copied().collect::<Vec<_>>();
            assert_slice_close(&seq_norm_vec, &par_norm_vec, 1.0e-6);
            assert_slice_close(&par_norm_vec, &default_norm_vec, 1.0e-6);

            let seq_k = seq.k().unwrap();
            let par_k = par.k().unwrap();
            let default_k = default.k().unwrap();
            assert_slice_close(seq_k, par_k, 1.0e-8);
            assert_slice_close(par_k, default_k, 1.0e-8);

            let seq_chi = seq.chi().unwrap();
            let par_chi = par.chi().unwrap();
            let default_chi = default.chi().unwrap();
            assert_slice_close(seq_chi, par_chi, 1.0e-6);
            assert_slice_close(par_chi, default_chi, 1.0e-6);

            let seq_chir_imag = seq.chir_imag().unwrap();
            let par_chir_imag = par.chir_imag().unwrap();
            let default_chir_imag = default.chir_imag().unwrap();
            assert_slice_close(seq_chir_imag.as_slice(), par_chir_imag.as_slice(), 1.0e-6);
            assert_slice_close(
                par_chir_imag.as_slice(),
                default_chir_imag.as_slice(),
                1.0e-6,
            );
        }
    }
}
