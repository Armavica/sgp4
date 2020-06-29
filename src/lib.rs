mod deep_space;
pub mod model;
mod near_earth;
mod propagator;
mod third_body;
pub mod tle;

pub use propagator::Constants;
pub use propagator::Error;
pub use propagator::Orbit;
pub use propagator::Prediction;
pub use propagator::Result;

impl Orbit {
    pub fn from_kozai_elements(
        geopotential: &model::Geopotential,
        inclination: f64,
        right_ascension: f64,
        eccentricity: f64,
        argument_of_perigee: f64,
        mean_anomaly: f64,
        kozai_mean_motion: f64,
    ) -> Result<Self> {
        if kozai_mean_motion <= 0.0 {
            Err(Error::new("the Kozai mean motion must be positive"))
        } else {
            let mean_motion = {
                // a₁ = (kₑ / n₀)²ᐟ³
                let a1 = (geopotential.ke / kozai_mean_motion).powf(2.0 / 3.0);

                //      3      3 cos²I₀
                // p₂ = - J₂ -----------
                //      4    (1 − e₀²)³ᐟ²
                let p2 = 0.75 * geopotential.j2 * (3.0 * inclination.cos().powi(2) - 1.0)
                    / (1.0 - eccentricity.powi(2)).powf(3.0 / 2.0);

                // 𝛿₁ = p₂ / a₁²
                let d1 = p2 / a1.powi(2);

                // 𝛿₀ = p₂ / (a₁ (1 - ¹/₃ 𝛿₁ - 𝛿₁² - ¹³⁴/₈₁ 𝛿₁³))²
                let d0 = p2
                    / (a1 * (1.0 - d1.powi(2) - d1 * (1.0 / 3.0 + 134.0 * d1.powi(2) / 81.0)))
                        .powi(2);

                //         n₀
                // n₀" = ------
                //       1 + 𝛿₀
                kozai_mean_motion / (1.0 + d0)
            };
            if mean_motion <= 0.0 {
                Err(Error::new("the Brouwer mean motion must be positive"))
            } else {
                Ok(Orbit {
                    inclination: inclination,
                    right_ascension: right_ascension,
                    eccentricity: eccentricity,
                    argument_of_perigee: argument_of_perigee,
                    mean_anomaly: mean_anomaly,
                    mean_motion: mean_motion,
                })
            }
        }
    }
}

// geopotential: the gravity model to use in calculations
// t0: years since UTC 1 January 2000 12h00 t₀
// drag_term: the radiation pressure coefficient B*, in earth radii⁻¹
// inclination_0: the angle between the equator and the orbit plane I₀, in rad
// right_ascension: the angle between vernal equinox and the point where
//                  the orbit crosses the equatorial plane Ω₀, in rad
// eccentricity_0: the shape of the orbit e₀
// argument_of_perigee: the angle between the ascending node and the orbit's
//                      point of closest approach to the earth ω₀, in rad
// mean_anomaly: the angle of the satellite location measured from perigee M₀, in rad
// mean_motion: mean number of orbits per day (Kozai mean motion) n₀, in rad.min⁻¹
impl<'a> Constants<'a> {
    pub fn new(
        geopotential: &'a model::Geopotential,
        epoch_to_sidereal_time: impl Fn(f64) -> f64,
        t0: f64,
        drag_term: f64,
        orbit_0: Orbit,
    ) -> Result<Self> {
        if orbit_0.eccentricity < 0.0 || orbit_0.eccentricity >= 1.0 {
            Err(Error::new("the eccentricity must be in the range [0, 1["))
        } else {
            // p₀ = cos I₀
            let p0 = orbit_0.inclination.cos();

            // p₁ = 1 − e₀²
            let p1 = 1.0 - orbit_0.eccentricity.powi(2);

            // k₆ = 3 p₀² - 1
            let k6 = 3.0 * p0.powi(2) - 1.0;

            // a₀" = (kₑ / n₀")²ᐟ³
            let a0 = (geopotential.ke / orbit_0.mean_motion).powf(2.0 / 3.0);

            // p₃ = a₀" (1 - e₀)
            let p3 = a0 * (1.0 - orbit_0.eccentricity);

            // perigee = aₑ (p₃ - 1)
            let perigee = geopotential.ae * (p3 - 1.0);

            // p₄ = │ 20             if perigee < 98
            //      │ (perigee - 78) if 98 ≤ perigee < 156
            //      │ 78             otherwise
            // s = p₄ / aₑ + 1
            // p₅ = ((120 - p₄) / aₑ)⁴
            let (s, p5) = {
                let p4 = if perigee < 98.0 {
                    20.0
                } else if perigee < 156.0 {
                    perigee - 78.0
                } else {
                    78.0
                };
                (
                    p4 / geopotential.ae + 1.0,
                    ((120.0 - p4) / geopotential.ae).powi(4),
                )
            };

            // ξ = 1 / (a₀" - s)
            let xi = 1.0 / (a0 - s);

            // p₆ = p₅ ξ⁴
            let p6 = p5 * xi.powi(4);

            // η = a₀" e₀ ξ
            let eta = a0 * orbit_0.eccentricity * xi;

            // p₇ = |1 - η²|
            let p7 = (1.0 - eta.powi(2)).abs();

            // p₈ = p₆ / p₇⁷ᐟ²
            let p8 = p6 / p7.powf(3.5);

            // C₁ = B* p₈ n₀" (a₀" (1 + ³/₂ η² + e₀ η (4 + η²))
            //      + ³/₈ J₂ ξ k₆ (8 + 3 η² (8 + η²)) / p₇)
            let c1 = drag_term
                * (p8
                    * orbit_0.mean_motion
                    * (a0
                        * (1.0
                            + 1.5 * eta.powi(2)
                            + orbit_0.eccentricity * eta * (4.0 + eta.powi(2)))
                        + 0.375 * geopotential.j2 * xi / p7
                            * k6
                            * (8.0 + 3.0 * eta.powi(2) * (8.0 + eta.powi(2)))));

            // p₉ = (a₀" p₁)⁻²
            let p9 = 1.0 / (a0 * p1).powi(2);

            // β₀ = p₁¹ᐟ²
            let b0 = p1.sqrt();

            // p₁₀ = ³/₂ J₂ p₉ n₀"
            let p10 = 1.5 * geopotential.j2 * p9 * orbit_0.mean_motion;

            // p₁₁ = ¹/₂ p₁₀ J₂ p₉
            let p11 = 0.5 * p10 * geopotential.j2 * p9;

            // p₁₂ = - ¹⁵/₃₂ J₄ p₉² n₀"
            let p12 = -0.46875 * geopotential.j4 * p9.powi(2) * orbit_0.mean_motion;

            // p₁₃ = - p₁₀ p₀ + (¹/₂ p₁₁ (4 - 19 p₀²) + 2 p₁₂ (3 - 7 p₀²)) p₀
            let p13 = -p10 * p0
                + (0.5 * p11 * (4.0 - 19.0 * p0.powi(2)) + 2.0 * p12 * (3.0 - 7.0 * p0.powi(2)))
                    * p0;

            // k₁₄ = - ¹/₂ p₁₀ (1 - 5 p₀²) + ¹/₁₆ p₁₁ (7 - 114 p₀² + 395 p₀⁴)
            let k14 = -0.5 * p10 * (1.0 - 5.0 * p0.powi(2))
                + 0.0625 * p11 * (7.0 - 114.0 * p0.powi(2) + 395.0 * p0.powi(4))
                + p12 * (3.0 - 36.0 * p0.powi(2) + 49.0 * p0.powi(4));

            // p₁₄ = n₀" + ¹/₂ p₁₀ β₀ k₆ + ¹/₁₆ p₁₁ β₀ (13 - 78 p₀² + 137 p₀⁴)
            let p14 = orbit_0.mean_motion
                + 0.5 * p10 * b0 * k6
                + 0.0625 * p11 * b0 * (13.0 - 78.0 * p0.powi(2) + 137.0 * p0.powi(4));

            //
            // C₄ = 2 n₀" p₈ a₀" p₁ [
            //      η (2 + ¹/₂ η²)
            //      + e₀ (¹/₂ + 2 η²)
            //      - J₂ ξ / (a p₇) (-3 k₆ (1 - 2 e₀ η + η² (³/₂ - ¹/₂ e₀ η))
            //      + ³/₄ (1 - p₀²) (2 η² - e₀ η (1 + η²)) cos 2 ω₀]
            let c4 = 2.0
                * orbit_0.mean_motion
                * p8
                * a0
                * p1
                * (eta * (2.0 + 0.5 * eta.powi(2))
                    + orbit_0.eccentricity * (0.5 + 2.0 * eta.powi(2))
                    - geopotential.j2 * xi / (a0 * p7)
                        * (-3.0
                            * k6
                            * (1.0 - 2.0 * orbit_0.eccentricity * eta
                                + eta.powi(2) * (1.5 - 0.5 * orbit_0.eccentricity * eta))
                            + 0.75
                                * (1.0 - p0.powi(2))
                                * (2.0 * eta.powi(2)
                                    - orbit_0.eccentricity * eta * (1.0 + eta.powi(2)))
                                * (2.0 * orbit_0.argument_of_perigee).cos()));

            // k₀ = - ⁷/₂ p₁ p₁₀ p₀ C₁
            let k0 = 3.5 * p1 * (-p10 * p0) * c1;

            // k₁ = ³/₂ C₁
            let k1 = 1.5 * c1;

            if orbit_0.mean_motion > 2.0 * model::PI / 255.0 {
                Ok(near_earth::constants(
                    geopotential,
                    drag_term,
                    orbit_0,
                    p0,
                    a0,
                    s,
                    xi,
                    eta,
                    c1,
                    c4,
                    k0,
                    k1,
                    k6,
                    k14,
                    p1,
                    p3,
                    p6,
                    p8,
                    p13,
                    p14,
                ))
            } else {
                Ok(deep_space::constants(
                    geopotential,
                    epoch_to_sidereal_time,
                    t0,
                    drag_term,
                    orbit_0,
                    p0,
                    a0,
                    c1,
                    b0,
                    c4,
                    k0,
                    k1,
                    k14,
                    p1,
                    p13,
                    p14,
                ))
            }
        }
    }

    pub fn from_tle(tle: &tle::Tle) -> Result<Self> {
        Constants::new(
            &model::WGS84,
            model::iau_epoch_to_sidereal_time,
            tle.epoch(),
            tle.drag_term,
            Orbit::from_kozai_elements(
                &model::WGS72,
                tle.inclination * (model::PI / 180.0),
                tle.right_ascension * (model::PI / 180.0),
                tle.eccentricity,
                tle.argument_of_perigee * (model::PI / 180.0),
                tle.mean_anomaly * (model::PI / 180.0),
                tle.mean_motion * (model::PI / 720.0),
            )?,
        )
    }

    pub fn from_tle_afspc_compatibility_mode(tle: &tle::Tle) -> Result<Self> {
        Constants::new(
            &model::WGS72,
            model::afspc_epoch_to_sidereal_time,
            tle.epoch_afspc_compatibility_mode(),
            tle.drag_term,
            Orbit::from_kozai_elements(
                &model::WGS72,
                tle.inclination * (model::PI / 180.0),
                tle.right_ascension * (model::PI / 180.0),
                tle.eccentricity,
                tle.argument_of_perigee * (model::PI / 180.0),
                tle.mean_anomaly * (model::PI / 180.0),
                tle.mean_motion * (model::PI / 720.0),
            )?,
        )
    }

    pub fn initial_state(&self) -> Option<deep_space::ResonanceState> {
        match &self.method {
            propagator::Method::NearEarth { .. } => None,
            propagator::Method::DeepSpace { resonant, .. } => match resonant {
                propagator::Resonant::No { .. } => None,
                propagator::Resonant::Yes { lambda_0, .. } => Some(
                    deep_space::ResonanceState::new(self.orbit_0.mean_motion, *lambda_0),
                ),
            },
        }
    }

    pub fn propagate_from_state(
        &self,
        t: f64,
        state: Option<&mut deep_space::ResonanceState>,
        afspc_compatibility_mode: bool,
    ) -> Result<Prediction> {
        // p₂₁ = Ω₀ + Ω̇ t + k₀ t²
        let p21 = self.orbit_0.right_ascension + self.right_ascension_dot * t + self.k0 * t.powi(2);

        // p₂₂ = ω₀ + ω̇ t
        let p22 = self.orbit_0.argument_of_perigee + self.argument_of_perigee_dot * t;
        let (orbit, a, l, p30, p31, p32, p33, p34) = match &self.method {
            propagator::Method::NearEarth {
                a0,
                k2,
                k3,
                k4,
                k5,
                k6,
                full,
            } => {
                assert!(
                    state.is_none(),
                    "state must be None with a near-earth propagator",
                );
                self.near_earth_orbital_elements(*a0, *k2, *k3, *k4, *k5, *k6, full, t, p21, p22)
            }
            propagator::Method::DeepSpace {
                eccentricity_dot,
                inclination_dot,
                solar_perturbations,
                lunar_perturbations,
                resonant,
            } => self.deep_space_orbital_elements(
                *eccentricity_dot,
                *inclination_dot,
                solar_perturbations,
                lunar_perturbations,
                resonant,
                state,
                t,
                p21,
                p22,
                afspc_compatibility_mode,
            ),
        }?;

        // p₂₇ = 1 / (a (1 - e²))
        let p27 = 1.0 / (a * (1.0 - orbit.eccentricity.powi(2)));

        // aₓₙ = e cos ω
        let axn = orbit.eccentricity * orbit.argument_of_perigee.cos();

        // aᵧₙ = e sin ω + p₂₇ p₃₀
        let ayn = orbit.eccentricity * orbit.argument_of_perigee.sin() + p27 * p30;

        // p₃₅ = 𝕃 + ω + p₂₇ p₃₃ aₓₙ rem 2π
        let p35 = (l + orbit.argument_of_perigee + p27 * p33 * axn) % (2.0 * model::PI);

        // (E + ω)₀ = p₃₅
        let mut ew = p35;
        for _ in 0..10 {
            //             p₃₅ - aᵧₙ cos (E + ω)ᵢ + aₓₙ sin (E + ω)ᵢ - (E + ω)ᵢ
            // Δ(E + ω)ᵢ = ---------------------------------------------------
            //                   1 - cos (E + ω)ᵢ aₓₙ - sin (E + ω)ᵢ aᵧₙ
            let delta = (p35 - ayn * ew.cos() + axn * ew.sin() - ew)
                / (1.0 - ew.cos() * axn - ew.sin() * ayn);

            if delta.abs() < 1.0e-12 {
                break;
            }

            // (E + ω)ᵢ₊₁ = (E + ω)ᵢ + Δ|[-0.95, 0.95]
            ew += if delta < -0.95 {
                -0.95
            } else if delta > 0.95 {
                0.95
            } else {
                delta
            };
        }

        // p₃₆ = aₓₙ² + aᵧₙ²
        let p36 = axn.powi(2) + ayn.powi(2);

        // pₗ = a (1 - p₃₆)
        let pl = a * (1.0 - p36);
        if pl < 0.0 {
            Err(Error::new("negative semi-latus rectum"))
        } else {
            // p₃₇ = aₓₙ cos(E + ω) + aᵧₙ sin(E + ω)
            let p37 = axn * ew.cos() + ayn * ew.sin();

            // p₃₈ = aₓₙ sin(E + ω) - aᵧₙ cos(E + ω)
            let p38 = axn * ew.sin() - ayn * ew.cos();

            // r = a (1 - p₃₇)
            let r = a * (1.0 - p37);

            // ṙ = a¹ᐟ² p₃₈ / r
            let r_dot = a.sqrt() * p38 / r;

            // β = (1 - p₃₆)¹ᐟ²
            let b = (1.0 - p36).sqrt();

            // p₃₉ = p₃₈ / (1 + β)
            let p39 = p38 / (1.0 + b);

            // p₄₀ = a / r (sin(E + ω) - aᵧₙ - aₓₙ p₃₉)
            let p40 = a / r * (ew.sin() - ayn - axn * p39);

            // p₄₁ = a / r (cos(E + ω) - aₓₙ + aᵧₙ p₃₉)
            let p41 = a / r * (ew.cos() - axn + ayn * p39);

            //           p₄₀
            // u = tan⁻¹ ---
            //           p₄₁
            let u = p40.atan2(p41);

            // p₄₂ = 2 p₄₁ p₄₀
            let p42 = 2.0 * p41 * p40;

            // p₄₃ = 1 - 2 p₄₀²
            let p43 = 1.0 - 2.0 * p40.powi(2);

            // p₄₄ = (¹/₂ J₂ / pₗ) / pₗ
            let p44 = 0.5 * self.geopotential.j2 / pl / pl;

            // rₖ = r (1 - ³/₂ p₄₄ β p₃₄) + ¹/₂ (¹/₂ J₂ / pₗ) p₃₁ p₄₃
            let rk = r * (1.0 - 1.5 * p44 * b * p34)
                + 0.5 * (0.5 * self.geopotential.j2 / pl) * p31 * p43;

            // uₖ = u - ¹/₄ p₄₄ p₃₂ p₄₂
            let uk = u - 0.25 * p44 * p32 * p42;

            // Ωₖ = Ω + ³/₂ p₄₄ cos I p₄₂
            let right_ascension_k =
                orbit.right_ascension + 1.5 * p44 * orbit.inclination.cos() * p42;

            // Iₖ = I + ³/₂ p₄₄ cos I sin I p₄₃
            let inclination_k = orbit.inclination
                + 1.5 * p44 * orbit.inclination.cos() * orbit.inclination.sin() * p43;

            // ṙₖ = ṙ + n (¹/₂ J₂ / pₗ) p₃₁ / kₑ
            let rk_dot = r_dot
                - orbit.mean_motion * (0.5 * self.geopotential.j2 / pl) * p31 * p42
                    / self.geopotential.ke;

            // rḟₖ = pₗ¹ᐟ² / r + n (¹/₂ J₂ / pₗ) (p₃₁ p₄₃ + ³/₂ p₃₄) / kₑ
            let rfk_dot = pl.sqrt() / r
                + orbit.mean_motion * (0.5 * self.geopotential.j2 / pl) * (p31 * p43 + 1.5 * p34)
                    / self.geopotential.ke;

            // u₀ = - sin Ωₖ cos Iₖ sin uₖ + cos Ωₖ cos uₖ
            let u0 = -right_ascension_k.sin() * inclination_k.cos() * uk.sin()
                + right_ascension_k.cos() * uk.cos();
            // u₁ = cos Ωₖ cos Iₖ sin uₖ + sin Ωₖ cos uₖ
            let u1 = right_ascension_k.cos() * inclination_k.cos() * uk.sin()
                + right_ascension_k.sin() * uk.cos();
            // u₂ = sin Iₖ sin uₖ
            let u2 = inclination_k.sin() * uk.sin();
            Ok(Prediction {
                position: [
                    // r₀ = rₖ u₀ aₑ
                    rk * u0 * self.geopotential.ae,
                    // r₁ = rₖ u₁ aₑ
                    rk * u1 * self.geopotential.ae,
                    // r₂ = rₖ u₂ aₑ
                    rk * u2 * self.geopotential.ae,
                ],
                velocity: [
                    // ṙ₀ = (ṙₖ u₀ + rḟₖ (- sin Ωₖ cos Iₖ cos uₖ - cos Ωₖ sin uₖ)) aₑ kₑ / 60
                    (rk_dot * u0
                        + rfk_dot
                            * (-right_ascension_k.sin() * inclination_k.cos() * uk.cos()
                                - right_ascension_k.cos() * uk.sin()))
                        * (self.geopotential.ae * self.geopotential.ke / 60.0),
                    // ṙ₁ = (ṙₖ u₁ + rḟₖ (cos Ωₖ cos Iₖ cos uₖ - sin Ωₖ sin uₖ)) aₑ kₑ / 60
                    (rk_dot * u1
                        + rfk_dot
                            * (right_ascension_k.cos() * inclination_k.cos() * uk.cos()
                                - right_ascension_k.sin() * uk.sin()))
                        * (self.geopotential.ae * self.geopotential.ke / 60.0),
                    // ṙ₂ = (ṙₖ u₂ + rḟₖ (sin Iₖ cos uₖ)) aₑ kₑ / 60
                    (rk_dot * u2 + rfk_dot * (inclination_k.sin() * uk.cos()))
                        * (self.geopotential.ae * self.geopotential.ke / 60.0),
                ],
            })
        }
    }

    pub fn propagate(&self, t: f64) -> Result<Prediction> {
        self.propagate_from_state(t, self.initial_state().as_mut(), false)
    }

    pub fn propagate_afspc_compatibility_mode(&self, t: f64) -> Result<Prediction> {
        self.propagate_from_state(t, self.initial_state().as_mut(), true)
    }
}
