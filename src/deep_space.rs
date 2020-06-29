use crate::model;
use crate::propagator;
use crate::third_body;
use std::cmp::Ordering;

// eₛ = 0.01675
pub const SOLAR_ECCENTRICITY: f64 = 0.01675;

// eₗ = 0.05490
pub const LUNAR_ECCENTRICITY: f64 = 0.05490;

// nₛ = 1.19459 × 10⁻⁵ rad.min⁻¹
pub const SOLAR_MEAN_MOTION: f64 = 1.19459e-5;

// nₗ = 1.5835218 × 10⁻⁴ rad.min⁻¹
pub const LUNAR_MEAN_MOTION: f64 = 1.5835218e-4;

// Cₛ = 2.9864797 × 10⁻⁶ rad.min⁻¹
pub const SOLAR_PERTURBATION_COEFFICIENT: f64 = 2.9864797e-6;

// Cₗ = 4.7968065 × 10⁻⁷ rad.min⁻¹
pub const LUNAR_PERTURBATION_COEFFICIENT: f64 = 4.7968065e-7;

// |Δt| = 720 min
const DELTA_T: f64 = 720.0;

// λ₃₁ = 0.13130908
const LAMBDA31: f64 = 0.13130908;

// λ₂₂ = 2.8843198
const LAMBDA22: f64 = 2.8843198;

// λ₃₃ = 0.37448087
const LAMBDA33: f64 = 0.37448087;

// G₂₂ = 5.7686396
const G22: f64 = 5.7686396;

// G₃₂ = 0.95240898
const G32: f64 = 0.95240898;

// G₄₄ = 1.8014998
const G44: f64 = 1.8014998;

// G₅₂ = 1.0508330
const G52: f64 = 1.0508330;

// G₅₄ = 4.4108898
const G54: f64 = 4.4108898;

pub struct ResonanceState {
    t: f64,
    mean_motion: f64,
    lambda: f64,
}

pub fn constants<'a>(
    geopotential: &'a model::Geopotential,
    epoch_to_sidereal_time: impl Fn(f64) -> f64,
    t0: f64,
    drag_term: f64,
    orbit_0: propagator::Orbit,
    p0: f64,
    a0: f64,
    c1: f64,
    b0: f64,
    c4: f64,
    k0: f64,
    k1: f64,
    k14: f64,
    p1: f64,
    p13: f64,
    p14: f64,
) -> propagator::Constants<'a> {
    // t₁₉₀₀ = 365.25 (t₀ + 100)
    let t1900 = (t0 + 100.0) * 365.25;
    let (solar_perturbations, solar_dots) = third_body::perturbations_and_dots(
        orbit_0.inclination,
        orbit_0.eccentricity,
        orbit_0.argument_of_perigee,
        orbit_0.mean_motion,
        0.39785416,
        0.91744867,
        orbit_0.right_ascension.sin(),
        orbit_0.right_ascension.cos(),
        SOLAR_ECCENTRICITY,
        -0.98088458,
        0.1945905,
        SOLAR_PERTURBATION_COEFFICIENT,
        SOLAR_MEAN_MOTION,
        // Mₛ₀ = (6.2565837 + 0.017201977 t₁₉₀₀) rem 2π
        (6.2565837 + 0.017201977 * t1900) % (2.0 * model::PI),
        p1,
        b0,
    );

    // Ωₗₑ = 4.523602 - 9.2422029 × 10⁻⁴ t₁₉₀₀ rem 2π
    let lunar_right_ascension_epsilon = (4.5236020 - 9.2422029e-4 * t1900) % (2.0 * model::PI);

    // cos Iₗ = 0.91375164 - 0.03568096 Ωₗₑ
    let lunar_inclination_cosine = 0.91375164 - 0.03568096 * lunar_right_ascension_epsilon.cos();

    // sin Iₗ = (1 - cos²Iₗ)¹ᐟ²
    let lunar_inclination_sine = (1.0 - lunar_inclination_cosine.powi(2)).sqrt();

    // sin Ωₗ = 0.089683511 sin Ωₗₑ / sin Iₗ;
    let lunar_right_ascension_sine =
        0.089683511 * lunar_right_ascension_epsilon.sin() / lunar_inclination_sine;

    // cos Ωₗ = (1 - sin²Ωₗ)¹ᐟ²
    let lunar_right_ascension_cosine = (1.0 - lunar_right_ascension_sine.powi(2)).sqrt();

    //
    // ωₗ = 5.8351514 + 0.001944368 t₁₉₀₀
    //                     0.39785416 sin Ωₗₑ / sin Iₗ
    //      + tan⁻¹ ------------------------------------------ - Ωₗₑ
    //              cos Ωₗ cos Ωₗₑ + 0.91744867 sin Ωₗ sin Ωₗₑ
    let lunar_argument_of_perigee = 5.8351514
        + 0.001944368 * t1900
        + (0.39785416 * lunar_right_ascension_epsilon.sin() / lunar_inclination_sine).atan2(
            lunar_right_ascension_cosine * lunar_right_ascension_epsilon.cos()
                + 0.91744867 * lunar_right_ascension_sine * lunar_right_ascension_epsilon.sin(),
        )
        - lunar_right_ascension_epsilon;
    let (lunar_perturbations, lunar_dots) = third_body::perturbations_and_dots(
        orbit_0.inclination,
        orbit_0.eccentricity,
        orbit_0.argument_of_perigee,
        orbit_0.mean_motion,
        lunar_inclination_sine,
        lunar_inclination_cosine,
        // sin(Ω₀ - Ωₗ) = sin Ω₀ cos Ωₗ - cos Ω₀ sin Ωₗ
        orbit_0.right_ascension.sin() * lunar_right_ascension_cosine
            - orbit_0.right_ascension.cos() * lunar_right_ascension_sine,
        // cos(Ω₀ - Ωₗ) = cos Ωₗ cos Ω₀ + sin Ωₗ sin Ω₀
        lunar_right_ascension_cosine * orbit_0.right_ascension.cos()
            + lunar_right_ascension_sine * orbit_0.right_ascension.sin(),
        LUNAR_ECCENTRICITY,
        lunar_argument_of_perigee.sin(),
        lunar_argument_of_perigee.cos(),
        LUNAR_PERTURBATION_COEFFICIENT,
        LUNAR_MEAN_MOTION,
        // Mₗ₀ = (-1.1151842 + 0.228027132 t₁₉₀₀) rem 2π
        (-1.1151842 + 0.228027132 * t1900) % (2.0 * model::PI),
        p1,
        b0,
    );
    propagator::Constants {
        geopotential: geopotential,
        drag_term: drag_term,

        // Ω̇ = p₁₃ + (Ω̇ₛ + Ω̇ₗ)
        right_ascension_dot: p13 + (solar_dots.right_ascension + lunar_dots.right_ascension),

        // ω̇ = k₁₄ + (ω̇ₛ + ω̇ₗ)
        argument_of_perigee_dot: k14
            + (solar_dots.argument_of_perigee + lunar_dots.argument_of_perigee),

        // Ṁ = p₁₄ + (Ṁₛ + Ṁₗ)
        mean_anomaly_dot: p14 + (solar_dots.mean_anomaly + lunar_dots.mean_anomaly),
        c1: c1,
        c4: c4,
        k0: k0,
        k1: k1,
        method: propagator::Method::DeepSpace {
            eccentricity_dot: solar_dots.eccentricity + lunar_dots.eccentricity,
            inclination_dot: solar_dots.inclination + lunar_dots.inclination,
            solar_perturbations: solar_perturbations,
            lunar_perturbations: lunar_perturbations,
            resonant: if (orbit_0.mean_motion < 0.0052359877 && orbit_0.mean_motion > 0.0034906585)
                || (orbit_0.mean_motion >= 8.26e-3
                    && orbit_0.mean_motion <= 9.24e-3
                    && orbit_0.eccentricity >= 0.5)
            {
                let sidereal_time_0 = epoch_to_sidereal_time(t0);
                if orbit_0.mean_motion < 0.0052359877 && orbit_0.mean_motion > 0.0034906585 {
                    propagator::Resonant::Yes {
                        // λ₀ = M₀ + Ω₀ + ω₀ − θ₀ rem 2π
                        lambda_0: (orbit_0.mean_anomaly
                            + orbit_0.right_ascension
                            + orbit_0.argument_of_perigee
                            - sidereal_time_0)
                            % (2.0 * model::PI),

                        // λ̇₀ = p₁₄ + (k₁₄ + p₁₃) − θ̇ + (Ṁₛ + Ṁₗ) + (ω̇ₛ + ω̇ₗ) + (Ω̇ₛ + Ω̇ₗ) - n₀"
                        lambda_dot_0: p14 + (k14 + p13) - model::SIDEREAL_SPEED
                            + (solar_dots.mean_anomaly + lunar_dots.mean_anomaly)
                            + (solar_dots.argument_of_perigee + lunar_dots.argument_of_perigee)
                            + (solar_dots.right_ascension + lunar_dots.right_ascension)
                            - orbit_0.mean_motion,
                        sidereal_time_0: sidereal_time_0,
                        resonance: {
                            // p₁₆ = 3 (n / a₀")²
                            let p16 = 3.0 * (orbit_0.mean_motion / a0).powi(2);
                            propagator::Resonance::OneDay {
                                // 𝛿ᵣ₁ = p₁₆ (¹⁵/₁₆ sin²I₀ (1 + 3 p₀) - ³/₄ (1 + p₀))
                                //           (1 + 2 e₀²) 2.1460748 × 10⁻⁶ / a₀"²
                                dr1: p16
                                    * (0.9375
                                        * orbit_0.inclination.sin().powi(2)
                                        * (1.0 + 3.0 * p0)
                                        - 0.75 * (1.0 + p0))
                                    * (1.0 + 2.0 * orbit_0.eccentricity.powi(2))
                                    * 2.1460748e-6
                                    / a0,

                                // 𝛿ᵣ₂ = 2 p₁₆ (³/₄ (1 + p₀)²)
                                //      (1 + e₀² (- ⁵/₂ + ¹³/₁₆ e₀²)) 1.7891679 × 10⁻⁶
                                dr2: 2.0
                                    * p16
                                    * (0.75 * (1.0 + p0).powi(2))
                                    * (1.0
                                        + orbit_0.eccentricity.powi(2)
                                            * (-2.5 + 0.8125 * orbit_0.eccentricity.powi(2)))
                                    * 1.7891679e-6,

                                // 𝛿ᵣ₃ = 3 p₁₆ (¹⁵/₈ (1 + p₀)³) (1 + e₀² (- 6 + 6.60937 e₀²))
                                //       2.2123015 × 10⁻⁷ / a₀"²
                                dr3: 3.0
                                    * p16
                                    * (1.875 * (1.0 + p0).powi(3))
                                    * (1.0
                                        + orbit_0.eccentricity.powi(2)
                                            * (-6.0 + 6.60937 * orbit_0.eccentricity.powi(2)))
                                    * 2.2123015e-7
                                    / a0,
                            }
                        },
                    }
                } else {
                    propagator::Resonant::Yes {
                        // λ₀ = M₀ + 2 Ω₀ − 2 θ₀ rem 2π
                        lambda_0: (orbit_0.mean_anomaly
                            + orbit_0.right_ascension
                            + orbit_0.right_ascension
                            - sidereal_time_0
                            - sidereal_time_0)
                            % (2.0 * model::PI),

                        // λ̇₀ = p₁₄ + (Ṁₛ + Ṁₗ) + 2 (p₁₃ + (Ω̇ₛ + Ω̇ₗ) - θ̇) - n₀"
                        lambda_dot_0: p14
                            + (solar_dots.mean_anomaly + lunar_dots.mean_anomaly)
                            + 2.0
                                * (p13 + (solar_dots.right_ascension + lunar_dots.right_ascension)
                                    - model::SIDEREAL_SPEED)
                            - orbit_0.mean_motion,
                        sidereal_time_0: sidereal_time_0,
                        resonance: {
                            // p₁₇ = 3 n₀"² / a₀"²
                            let p17 = 3.0 * orbit_0.mean_motion.powi(2) * (1.0 / a0).powi(2);

                            // p₁₈ = p₁₇ / a₀"
                            let p18 = p17 * (1.0 / a0);

                            // p₁₉ = p₁₈ / a₀"
                            let p19 = p18 * (1.0 / a0);

                            // p₂₀ = p₁₉ / a₀"
                            let p20 = p19 * (1.0 / a0);

                            // F₂₂₀ = ³/₄ (1 + 2 p₀ + p₀²)
                            let f220 = 0.75 * (1.0 + 2.0 * p0 + p0.powi(2));

                            // G₂₁₁ = │ 3.616 - 13.247 e₀ + 16.29 e₀²                          if e₀ ≤ 0.65
                            //        │ - 72.099 + 331.819 e₀ - 508.738 e₀² + 266.724 e₀³      otherwise
                            // G₃₁₀ = │ - 19.302 + 117.39 e₀ - 228.419 e₀² + 156.591 e₀³       if e₀ ≤ 0.65
                            //        │ - 346.844 + 1582.851 e₀ - 2415.925 e₀² + 1246.113 e₀³  otherwise
                            // G₃₂₂ = │ - 18.9068 + 109.7927 e₀ - 214.6334 e₀² + 146.5816 e₀³  if e₀ ≤ 0.65
                            //        │ - 342.585 + 1554.908 e₀ - 2366.899 e₀² + 1215.972 e₀³  otherwise
                            // G₄₁₀ = │ - 41.122 + 242.694 e₀ - 471.094 e₀² + 313.953 e₀³      if e₀ ≤ 0.65
                            //        │ - 1052.797 + 4758.686 e₀ - 7193.992 e₀² + 3651.957 e₀³ otherwise
                            // G₄₂₂ = │ - 146.407 + 841.88 e₀ - 1629.014 e₀² + 1083.435 e₀³    if e₀ ≤ 0.65
                            //        │ - 3581.69 + 16178.11 e₀ - 24462.77 e₀² + 12422.52 e₀³  otherwise
                            let (g211, g310, g322, g410, g422) = if orbit_0.eccentricity <= 0.65 {
                                (
                                    3.616 - 13.247 * orbit_0.eccentricity
                                        + 16.29 * orbit_0.eccentricity.powi(2),
                                    -19.302 + 117.39 * orbit_0.eccentricity
                                        - 228.419 * orbit_0.eccentricity.powi(2)
                                        + 156.591 * orbit_0.eccentricity.powi(3),
                                    -18.9068 + 109.7927 * orbit_0.eccentricity
                                        - 214.6334 * orbit_0.eccentricity.powi(2)
                                        + 146.5816 * orbit_0.eccentricity.powi(3),
                                    -41.122 + 242.694 * orbit_0.eccentricity
                                        - 471.094 * orbit_0.eccentricity.powi(2)
                                        + 313.953 * orbit_0.eccentricity.powi(3),
                                    -146.407 + 841.88 * orbit_0.eccentricity
                                        - 1629.014 * orbit_0.eccentricity.powi(2)
                                        + 1083.435 * orbit_0.eccentricity.powi(3),
                                )
                            } else {
                                (
                                    -72.099 + 331.819 * orbit_0.eccentricity
                                        - 508.738 * orbit_0.eccentricity.powi(2)
                                        + 266.724 * orbit_0.eccentricity.powi(3),
                                    -346.844 + 1582.851 * orbit_0.eccentricity
                                        - 2415.925 * orbit_0.eccentricity.powi(2)
                                        + 1246.113 * orbit_0.eccentricity.powi(3),
                                    -342.585 + 1554.908 * orbit_0.eccentricity
                                        - 2366.899 * orbit_0.eccentricity.powi(2)
                                        + 1215.972 * orbit_0.eccentricity.powi(3),
                                    -1052.797 + 4758.686 * orbit_0.eccentricity
                                        - 7193.992 * orbit_0.eccentricity.powi(2)
                                        + 3651.957 * orbit_0.eccentricity.powi(3),
                                    -3581.69 + 16178.11 * orbit_0.eccentricity
                                        - 24462.77 * orbit_0.eccentricity.powi(2)
                                        + 12422.52 * orbit_0.eccentricity.powi(3),
                                )
                            };

                            // G₅₂₀ = │ - 532.114 + 3017.977 e₀ - 5740.032 e₀² + 3708.276 e₀³ if e₀ ≤ 0.65
                            //        │ 1464.74 - 4664.75 e₀ + 3763.64 e₀²                    if 0.65 < e₀ < 0.715
                            //        │ - 5149.66 + 29936.92 e₀ - 54087.36 e₀² + 31324.56 e₀³ otherwise
                            let g520 = if orbit_0.eccentricity <= 0.65 {
                                -532.114 + 3017.977 * orbit_0.eccentricity
                                    - 5740.032 * orbit_0.eccentricity.powi(2)
                                    + 3708.276 * orbit_0.eccentricity.powi(3)
                            } else if orbit_0.eccentricity < 0.715 {
                                1464.74 - 4664.75 * orbit_0.eccentricity
                                    + 3763.64 * orbit_0.eccentricity.powi(2)
                            } else {
                                -5149.66 + 29936.92 * orbit_0.eccentricity
                                    - 54087.36 * orbit_0.eccentricity.powi(2)
                                    + 31324.56 * orbit_0.eccentricity.powi(3)
                            };

                            // G₅₃₂ = │ - 853.666 + 4690.25 e₀ - 8624.77 e₀² + 5341.4 e₀³          if e₀ < 0.7
                            //        │ - 40023.88 + 170470.89 e₀ - 242699.48 e₀² + 115605.82 e₀³  otherwise
                            // G₅₂₁ = │ - 822.71072 + 4568.6173 e₀ - 8491.4146 e₀² + 5337.524 e₀³  if e₀ < 0.7
                            //        │ - 51752.104 + 218913.95 e₀ - 309468.16 e₀² + 146349.42 e₀³ otherwise
                            // G₅₃₃ = │ - 919.2277 + 4988.61 e₀ - 9064.77 e₀² + 5542.21 e₀³        if e₀ < 0.7
                            //        │ - 37995.78 + 161616.52 e₀ - 229838.2 e₀² + 109377.94 e₀³   otherwise
                            let (g532, g521, g533) = if orbit_0.eccentricity < 0.7 {
                                (
                                    -853.666 + 4690.25 * orbit_0.eccentricity
                                        - 8624.77 * orbit_0.eccentricity.powi(2)
                                        + 5341.4 * orbit_0.eccentricity.powi(3),
                                    -822.71072 + 4568.6173 * orbit_0.eccentricity
                                        - 8491.4146 * orbit_0.eccentricity.powi(2)
                                        + 5337.524 * orbit_0.eccentricity.powi(3),
                                    -919.2277 + 4988.61 * orbit_0.eccentricity
                                        - 9064.77 * orbit_0.eccentricity.powi(2)
                                        + 5542.21 * orbit_0.eccentricity.powi(3),
                                )
                            } else {
                                (
                                    -40023.88 + 170470.89 * orbit_0.eccentricity
                                        - 242699.48 * orbit_0.eccentricity.powi(2)
                                        + 115605.82 * orbit_0.eccentricity.powi(3),
                                    -51752.104 + 218913.95 * orbit_0.eccentricity
                                        - 309468.16 * orbit_0.eccentricity.powi(2)
                                        + 146349.42 * orbit_0.eccentricity.powi(3),
                                    -37995.78 + 161616.52 * orbit_0.eccentricity
                                        - 229838.2 * orbit_0.eccentricity.powi(2)
                                        + 109377.94 * orbit_0.eccentricity.powi(3),
                                )
                            };

                            propagator::Resonance::HalfDay {
                                // D₂₂₀₋₁ = p₁₇ 1.7891679 × 10⁻⁶ F₂₂₀ (- 0.306 - 0.44 (e₀ - 0.64))
                                d2201: p17
                                    * 1.7891679e-6
                                    * f220
                                    * (-0.306 - (orbit_0.eccentricity - 0.64) * 0.44),

                                // D₂₂₁₁ = p₁₇ 1.7891679 × 10⁻⁶ (³/₂ sin²I₀) G₂₁₁
                                d2211: p17
                                    * 1.7891679e-6
                                    * (1.5 * orbit_0.inclination.sin().powi(2))
                                    * g211,

                                // D₃₂₁₀ = p₁₈ 3.7393792 × 10⁻⁷ (¹⁵/₈ sin I₀ (1 - 2 p₀ - 3 p₀²)) G₃₁₀
                                d3210: p18
                                    * 3.7393792e-7
                                    * (1.875
                                        * orbit_0.inclination.sin()
                                        * (1.0 - 2.0 * p0 - 3.0 * p0.powi(2)))
                                    * g310,

                                // D₃₂₂₂ = p₁₈ 3.7393792 × 10⁻⁷ (- ¹⁵/₈ sin I₀ (1 + 2 p₀ - 3 p₀²)) G₃₂₂
                                d3222: p18
                                    * 3.7393792e-7
                                    * (-1.875
                                        * orbit_0.inclination.sin()
                                        * (1.0 + 2.0 * p0 - 3.0 * p0.powi(2)))
                                    * g322,

                                // D₄₄₁₀ = 2 p₁₉ 7.3636953 × 10⁻⁹ (35 sin²I₀ F₂₂₀) G₄₁₀
                                d4410: 2.0
                                    * p19
                                    * 7.3636953e-9
                                    * (35.0 * orbit_0.inclination.sin().powi(2) * f220)
                                    * g410,

                                // D₄₄₂₂ = 2 p₁₉ 7.3636953 × 10⁻⁹ (³¹⁵/₈ sin⁴I₀) G₄₂₂
                                d4422: 2.0
                                    * p19
                                    * 7.3636953e-9
                                    * (39.375 * orbit_0.inclination.sin().powi(4))
                                    * g422,

                                // D₅₂₂₀ = p₂₀ 1.1428639 × 10⁻⁷ (³¹⁵/₃₂ sin I₀
                                //         (sin²I₀ (1 - 2 p₀ - 5 p₀²)
                                //         + 0.33333333 (- 2 + 4 p₀ + 6 p₀²))) G₅₂₀
                                d5220: p20
                                    * 1.1428639e-7
                                    * (9.84375
                                        * orbit_0.inclination.sin()
                                        * (orbit_0.inclination.sin().powi(2)
                                            * (1.0 - 2.0 * p0 - 5.0 * p0.powi(2))
                                            + 0.33333333 * (-2.0 + 4.0 * p0 + 6.0 * p0.powi(2))))
                                    * g520,

                                // D₅₂₃₂ = p₂₀ 1.1428639 × 10⁻⁷ (sin I₀
                                //         (4.92187512 sin²I₀ (- 2 - 4 p₀ + 10 p₀²)
                                //         + 6.56250012 (1 + p₀ - 3 p₀²))) G₅₃₂
                                d5232: p20
                                    * 1.1428639e-7
                                    * (orbit_0.inclination.sin()
                                        * (4.92187512
                                            * orbit_0.inclination.sin().powi(2)
                                            * (-2.0 - 4.0 * p0 + 10.0 * p0.powi(2))
                                            + 6.56250012 * (1.0 + 2.0 * p0 - 3.0 * p0.powi(2))))
                                    * g532,

                                // D₅₄₂₁ = 2 p₂₀ 2.1765803 × 10⁻⁹ (⁹⁴⁵/₃₂ sin I₀
                                //         (2 - 8 p₀ + p₀² (- 12 + 8 p₀ + 10 p₀²))) G₅₂₁
                                d5421: 2.0
                                    * p20
                                    * 2.1765803e-9
                                    * (29.53125
                                        * orbit_0.inclination.sin()
                                        * (2.0 - 8.0 * p0
                                            + p0.powi(2) * (-12.0 + 8.0 * p0 + 10.0 * p0.powi(2))))
                                    * g521,

                                // D₅₄₃₃ = 2 p₂₀ 2.1765803 × 10⁻⁹ (⁹⁴⁵/₃₂ sin I₀
                                //         (- 2 - 8 p₀ + p₀² (12 + 8 p₀ - 10 p₀²))) G₅₃₃
                                d5433: 2.0
                                    * p20
                                    * 2.1765803e-9
                                    * (29.53125
                                        * orbit_0.inclination.sin()
                                        * (-2.0 - 8.0 * p0
                                            + p0.powi(2) * (12.0 + 8.0 * p0 - 10.0 * p0.powi(2))))
                                    * g533,
                                k14: k14,
                            }
                        },
                    }
                }
            } else {
                propagator::Resonant::No { a0: a0 }
            },
        },
        orbit_0: orbit_0,
    }
}

impl ResonanceState {
    pub fn new(mean_motion_0: f64, lambda_0: f64) -> ResonanceState {
        ResonanceState {
            t: 0.0,
            mean_motion: mean_motion_0,
            lambda: lambda_0,
        }
    }

    pub fn integrate(
        &mut self,
        geopotential: &model::Geopotential,
        argument_of_perigee_0: f64,
        lambda_dot_0: f64,
        resonance: &propagator::Resonance,
        sidereal_time_0: f64,
        t: f64,
        p21: f64,
        p22: f64,
    ) -> (f64, f64) {
        if (self.t != 0.0 && self.t.is_sign_positive() != t.is_sign_positive())
            || t.abs() < self.t.abs()
        {
            panic!("the resonance integration state must be manually reset if the target times are non-monotonic");
        }
        // θ = θ₀ + 4.37526908801129966 × 10⁻³ t rem 2π
        let sidereal_time = (sidereal_time_0 + t * 4.37526908801129966e-3) % (2.0 * model::PI);
        let (delta_t, ordering) = if t > 0.0 {
            (DELTA_T, Ordering::Less)
        } else {
            (-DELTA_T, Ordering::Greater)
        };
        loop {
            // λ̇ᵢ = nᵢ + λ̇₀
            let lambda_dot = self.mean_motion + lambda_dot_0;
            let (ni_dot, ni_ddot) = match resonance {
                propagator::Resonance::OneDay { dr1, dr2, dr3 } => (
                    // ṅᵢ = 𝛿ᵣ₁ sin(λᵢ - λ₃₁) + 𝛿ᵣ₂ sin(2 (λᵢ - λ₂₂)) + 𝛿ᵣ₃ sin(3 (λᵢ - λ₃₃))
                    dr1 * (self.lambda - LAMBDA31).sin()
                        + dr2 * (2.0 * (self.lambda - LAMBDA22)).sin()
                        + dr3 * (3.0 * (self.lambda - LAMBDA33)).sin(),
                    // n̈ᵢ = (𝛿ᵣ₁ cos(λᵢ - λ₃₁) + 𝛿ᵣ₂ cos(2 (λᵢ - λ₂₂)) + 𝛿ᵣ₃ cos(3 (λᵢ - λ₃₃))) λ̇ᵢ
                    (dr1 * (self.lambda - LAMBDA31).cos()
                        + 2.0 * dr2 * (2.0 * (self.lambda - LAMBDA22)).cos()
                        + 3.0 * dr3 * (3.0 * (self.lambda - LAMBDA33)).cos())
                        * lambda_dot,
                ),
                propagator::Resonance::HalfDay {
                    d2201,
                    d2211,
                    d3210,
                    d3222,
                    d4410,
                    d4422,
                    d5220,
                    d5232,
                    d5421,
                    d5433,
                    k14,
                } => {
                    // ωᵢ = ω₀ + ω̇ tᵢ
                    let argument_of_perigee_i = argument_of_perigee_0 + k14 * self.t;
                    (
                        // ṅᵢ = Σ₍ₗₘₚₖ₎ Dₗₘₚₖ sin((l - 2 p) ωᵢ + m / 2 λᵢ - Gₗₘ)
                        // (l, m, p, k) ∈ {(2, 2, 0, -1), (2, 2, 1, 1), (3, 2, 1, 0),
                        //     (3, 2, 2, 2), (4, 4, 1, 0), (4, 4, 2, 2), (5, 2, 2, 0),
                        //     (5, 2, 3, 2), (5, 4, 2, 1), (5, 4, 3, 3)}
                        d2201 * (2.0 * argument_of_perigee_i + self.lambda - G22).sin()
                            + d2211 * (self.lambda - G22).sin()
                            + d3210 * (argument_of_perigee_i + self.lambda - G32).sin()
                            + d3222 * (-argument_of_perigee_i + self.lambda - G32).sin()
                            + d4410 * (2.0 * argument_of_perigee_i + 2.0 * self.lambda - G44).sin()
                            + d4422 * (2.0 * self.lambda - G44).sin()
                            + d5220 * (argument_of_perigee_i + self.lambda - G52).sin()
                            + d5232 * (-argument_of_perigee_i + self.lambda - G52).sin()
                            + d5421 * (argument_of_perigee_i + 2.0 * self.lambda - G54).sin()
                            + d5433 * (-argument_of_perigee_i + 2.0 * self.lambda - G54).sin(),
                        // n̈ᵢ = (Σ₍ₗₘₚₖ₎ m / 2 Dₗₘₚₖ cos((l - 2 p) ωᵢ + m / 2 λᵢ - Gₗₘ)) λ̇ᵢ
                        // (l, m, p, k) ∈ {(2, 2, 0, -1), (2, 2, 1, 1), (3, 2, 1, 0),
                        //     (3, 2, 2, 2), (4, 4, 1, 0), (4, 4, 2, 2), (5, 2, 2, 0),
                        //     (5, 2, 3, 2), (5, 4, 2, 1), (5, 4, 3, 3)}
                        (d2201 * (2.0 * argument_of_perigee_i + self.lambda - G22).cos()
                            + d2211 * (self.lambda - G22).cos()
                            + d3210 * (argument_of_perigee_i + self.lambda - G32).cos()
                            + d3222 * (-argument_of_perigee_i + self.lambda - G32).cos()
                            + d5220 * (argument_of_perigee_i + self.lambda - G52).cos()
                            + d5232 * (-argument_of_perigee_i + self.lambda - G52).cos()
                            + 2.0
                                * (d4410
                                    * (2.0 * argument_of_perigee_i + 2.0 * self.lambda - G44)
                                        .cos()
                                    + d4422 * (2.0 * self.lambda - G44).cos()
                                    + d5421
                                        * (argument_of_perigee_i + 2.0 * self.lambda - G54).cos()
                                    + d5433
                                        * (-argument_of_perigee_i + 2.0 * self.lambda - G54)
                                            .cos()))
                            * lambda_dot,
                    )
                }
            };
            if (t - delta_t)
                .partial_cmp(&self.t)
                .unwrap_or(Ordering::Equal)
                == ordering
            {
                return (
                    // p₂₆ = (kₑ / (nᵢ + ṅᵢ (t - tᵢ) + ¹/₂ n̈ᵢ (t - tᵢ)²))²ᐟ³
                    (geopotential.ke
                        / (self.mean_motion
                            + ni_dot * (t - self.t)
                            + ni_ddot * (t - self.t).powi(2) * 0.5))
                        .powf(2.0 / 3.0),
                    match resonance {
                        propagator::Resonance::OneDay { .. } => {
                            // p₂₇ = λᵢ + λ̇ᵢ (t - tᵢ) + ¹/₂ ṅᵢ (t - tᵢ)² - p₂₁ - p₂₂ + θ
                            self.lambda
                                + lambda_dot * (t - self.t)
                                + ni_dot * (t - self.t).powi(2) * 0.5
                                - p21
                                - p22
                                + sidereal_time
                        }
                        propagator::Resonance::HalfDay { .. } => {
                            // p₂₇ = λᵢ + λ̇ᵢ (t - tᵢ) + ¹/₂ ṅᵢ (t - tᵢ)² - 2 p₂₁ + 2 θ
                            self.lambda
                                + lambda_dot * (t - self.t)
                                + ni_dot * (t - self.t).powi(2) * 0.5
                                - 2.0 * p21
                                + 2.0 * sidereal_time
                        }
                    },
                );
            }

            // tᵢ₊₁ = tᵢ + Δt
            self.t += delta_t;

            // nᵢ₊₁ = nᵢ + ṅᵢ Δt + n̈ᵢ (Δt² / 2)
            self.mean_motion += ni_dot * delta_t + ni_ddot * (DELTA_T.powi(2) / 2.0);

            // λᵢ₊₁ = λᵢ + λ̇ᵢ Δt + ṅᵢ (Δt² / 2)
            self.lambda += lambda_dot * delta_t + ni_dot * (DELTA_T.powi(2) / 2.0);
        }
    }
}

impl<'a> propagator::Constants<'a> {
    pub fn deep_space_orbital_elements(
        &self,
        eccentricity_dot: f64,
        inclination_dot: f64,
        solar_perturbations: &third_body::Perturbations,
        lunar_perturbations: &third_body::Perturbations,
        resonant: &propagator::Resonant,
        state: Option<&mut ResonanceState>,
        t: f64,
        p21: f64,
        p22: f64,
        afspc_compatibility_mode: bool,
    ) -> propagator::Result<(propagator::Orbit, f64, f64, f64, f64, f64, f64, f64)> {
        let (p26, p27) = match resonant {
            propagator::Resonant::No { a0 } => {
                assert!(
                    state.is_none(),
                    "state must be None with a non-resonant deep-space propagator",
                );
                (
                    // p₂₆ = a₀"
                    *a0,
                    // p₂₇ = M₀ + Ṁ t
                    self.orbit_0.mean_anomaly + self.mean_anomaly_dot * t,
                )
            }
            propagator::Resonant::Yes {
                lambda_dot_0,
                sidereal_time_0,
                resonance,
                ..
            } => match state {
                Some(state) => state.integrate(
                    self.geopotential,
                    self.orbit_0.argument_of_perigee,
                    *lambda_dot_0,
                    resonance,
                    *sidereal_time_0,
                    t,
                    p21,
                    p22,
                ),
                _ => panic!("state cannot be None with a deep space propagator"),
            },
        };
        let (solar_delta_eccentricity, solar_delta_inclination, solar_delta_mean_motion, ls4, ls5) =
            solar_perturbations.long_period_periodic_effects(
                SOLAR_ECCENTRICITY,
                SOLAR_MEAN_MOTION,
                t,
            );
        let (lunar_delta_eccentricity, lunar_delta_inclination, lunar_delta_mean_motion, lp5, lp6) =
            lunar_perturbations.long_period_periodic_effects(
                LUNAR_ECCENTRICITY,
                LUNAR_MEAN_MOTION,
                t,
            );

        // I = I₀ + İ t + (δIₛ + δIₗ)
        let inclination = self.orbit_0.inclination
            + inclination_dot * t
            + (solar_delta_inclination + lunar_delta_inclination);
        let (right_ascension, argument_of_perigee) = if inclination >= 0.2 {
            (
                // Ω = p₂₁ + (lₛ₅ + lₗ₅) / sin I
                p21 + (ls5 + lp6) / inclination.sin(),
                // ω = p₂₂ + (lₛ₄ + lₗ₄) - cos I (lₛ₅ + lₗ₅) / sin I
                p22 + (ls4 + lp5) - inclination.cos() * ((ls5 + lp6) / inclination.sin()),
            )
        } else {
            //             sin I sin p₂₁ + (lₛ₅ + lₗ₅) cos p₂₁ + (δIₛ + δIₗ) cos I sin p₂₁
            // p₂₈ = tan⁻¹ -------------------------------------------------------------
            //             sin I cos p₂₁ - (lₛ₅ + lₗ₅) sin p₂₁ + (δIₛ + δIₗ) cos I cos p₂₁
            let p28 = (inclination.sin() * p21.sin()
                + ((ls5 + lp6) * p21.cos()
                    + (solar_delta_inclination + lunar_delta_inclination)
                        * inclination.cos()
                        * p21.sin()))
            .atan2(
                inclination.sin() * p21.cos()
                    + (-(ls5 + lp6) * p21.sin()
                        + (solar_delta_inclination + lunar_delta_inclination)
                            * inclination.cos()
                            * p21.cos()),
            );

            // Ω = │ p₂₈ + 2π if p₂₈ + π < p₂₁ rem 2π
            //     │ p₂₈ - 2π if p₂₈ - π > p₂₁ rem 2π
            //     │ p₂₈      otherwise
            let right_ascension = if p28 < p21 % (2.0 * model::PI) - model::PI {
                p28 + (2.0 * model::PI)
            } else if p28 > p21 % (2.0 * model::PI) + model::PI {
                p28 - (2.0 * model::PI)
            } else {
                p28
            };
            (
                right_ascension,
                // ω = │ p₂₂ + (lₛ₄ + lₗ₄) + cos I ((p₂₁ rem 2π) - Ω)
                //     │ - (δIₛ + δIₗ) (p₂₁ mod 2π) sin I             if AFSPC compatibility mode
                // ω = │ p₂₂ + (lₛ₄ + lₗ₄) + cos I ((p₂₁ rem 2π) - Ω)
                //     │ - (δIₛ + δIₗ) (p₂₁ rem 2π) sin I             otherwise
                p22 + (ls4 + lp5) + inclination.cos() * (p21 % (2.0 * model::PI) - right_ascension)
                    - (solar_delta_inclination + lunar_delta_inclination)
                        * if afspc_compatibility_mode {
                            p21.rem_euclid(2.0 * model::PI)
                        } else {
                            p21 % (2.0 * model::PI)
                        }
                        * inclination.sin(),
            )
        };

        // p₂₉ = e₀ + ė t - B* C₄ t
        let p29 = self.orbit_0.eccentricity + eccentricity_dot * t - self.drag_term * self.c4 * t;
        if p29 >= 1.0 || p29 < -0.001 {
            Err(propagator::Error::new("diverging eccentricity"))
        } else {
            // e = │ 10⁻⁶ + (δeₛ + δeₗ) if p₂₉ < 10⁻⁶
            //     │ p₂₉ + (δeₛ + δeₗ)  otherwise
            let eccentricity =
                (p29).max(1.0e-6) + (solar_delta_eccentricity + lunar_delta_eccentricity);
            if eccentricity < 0.0 || eccentricity > 1.0 {
                Err(propagator::Error::new("diverging perturbed eccentricity"))
            } else {
                // M = p₂₇ + (δMₛ + δMₗ)
                let mean_anomaly = p27 + (solar_delta_mean_motion + lunar_delta_mean_motion);

                // a = p₂₆ (1 - C₁ t)²
                let a = p26 * (1.0 - self.c1 * t).powi(2);
                Ok((
                    propagator::Orbit {
                        inclination: inclination,
                        right_ascension: right_ascension,
                        eccentricity: eccentricity,
                        argument_of_perigee: argument_of_perigee,
                        mean_anomaly: mean_anomaly,
                        // n = kₑ / a³ᐟ²
                        mean_motion: self.geopotential.ke / a.powf(1.5),
                    },
                    a,
                    // 𝕃 = M + n₀" k₁ t²
                    mean_anomaly + self.orbit_0.mean_motion * self.k1 * t.powi(2),
                    //         1 J₃
                    // p₃₀ = - - -- sin I
                    //         2 J₂
                    -0.5 * (self.geopotential.j3 / self.geopotential.j2) * inclination.sin(),
                    // p₃₁ = 1 - cos²I
                    1.0 - inclination.cos().powi(2),
                    // p₃₂ = k₄
                    7.0 * inclination.cos().powi(2) - 1.0,
                    //       │   1 J₃       3 + 5 cos I
                    // p₃₃ = │ - - -- sin I ----------- if |1 + cos I| > 1.5 × 10⁻¹²
                    //       │   4 J₂        1 + cos I
                    //       │   1 J₃       3 + 5 cos I
                    //       │ - - -- sin I ----------- otherwise
                    //       │   4 J₂       1.5 × 10⁻¹²
                    if (1.0 + inclination.cos()).abs() > 1.5e-12 {
                        -0.25
                            * (self.geopotential.j3 / self.geopotential.j2)
                            * inclination.sin()
                            * (3.0 + 5.0 * inclination.cos())
                            / (1.0 + inclination.cos())
                    } else {
                        -0.25
                            * (self.geopotential.j3 / self.geopotential.j2)
                            * inclination.sin()
                            * (3.0 + 5.0 * inclination.cos())
                            / 1.5e-12
                    },
                    // p₃₄ = k₆
                    3.0 * inclination.cos().powi(2) - 1.0,
                ))
            }
        }
    }
}
