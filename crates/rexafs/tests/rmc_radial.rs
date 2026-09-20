use rexafs::rmc::{radial_distribution, Atom, Configuration};

fn pair() -> Configuration {
    Configuration {
        atoms: vec![
            Atom {
                atomic_number: 29,
                position: [0.; 3],
            },
            Atom {
                atomic_number: 8,
                position: [2., 0., 0.],
            },
        ],
        cell: None,
    }
}

#[test]
fn finite_cluster_has_counts_without_an_invented_density() {
    let result = radial_distribution(&pair(), &[0], Some(8), &[0., 1., 3.]).unwrap();
    assert_eq!(result.neighbors.counts_per_absorber, vec![0., 1.]);
    assert_eq!(result.neighbors.coordination, 1.);
    assert_eq!(result.neighbors.mean, Some(2.));
    assert_eq!(result.neighbors.variance, Some(0.));
    assert!(result.g_r.is_none());
    assert!(result.density.is_none());
}

#[test]
fn periodic_density_and_exact_shell_volume_recover_coordination() {
    let mut configuration = pair();
    configuration.cell = Some([[10., 0., 0.], [0., 10., 0.], [0., 0., 10.]]);
    let result = radial_distribution(&configuration, &[0], Some(8), &[0., 1., 3.]).unwrap();
    let density = result.density.unwrap();
    assert!((density - 0.001).abs() < 1e-15);
    assert_eq!(result.independent_radius, Some(5.));
    let shell = 4. * std::f64::consts::PI / 3. * (27. - 1.);
    assert!((result.g_r.unwrap()[1] * density * shell - 1.).abs() < 1e-14);
    assert!(radial_distribution(&configuration, &[0], Some(26), &[0., 3.]).is_err());
}

#[test]
fn self_is_excluded_but_periodic_self_images_are_counted() {
    let configuration = Configuration {
        atoms: vec![Atom {
            atomic_number: 29,
            position: [0.; 3],
        }],
        cell: Some([[2., 0., 0.], [0., 2., 0.], [0., 0., 2.]]),
    };
    let result = radial_distribution(&configuration, &[0], Some(29), &[0., 1., 2.1]).unwrap();
    assert_eq!(result.neighbors.counts_per_absorber, vec![0., 6.]);
    assert_eq!(result.density, Some(0.125));
    assert_eq!(result.independent_radius, Some(1.));
}
