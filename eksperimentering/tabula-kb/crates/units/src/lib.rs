use core::fmt;

/// Fixed-point signed 96.32 (i128 with 32 fractional bits)
/// Deterministic arithmetic: multiplication and division truncate toward zero.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FixI128(pub i128);

impl FixI128 {
    pub const FRAC_BITS: u32 = 32;
    pub const ONE: Self = Self(1i128 << Self::FRAC_BITS);

    #[inline]
    pub fn from_i64(x: i64) -> Self { Self((x as i128) << Self::FRAC_BITS) }

    #[inline]
    pub const fn from_ratio(num: i128, den: i128) -> Self {
        Self((num << Self::FRAC_BITS) / den)
    }

    #[inline]
    pub fn mul(self, rhs: Self) -> Self { Self((self.0 * rhs.0) >> Self::FRAC_BITS) }

    #[inline]
    pub fn div(self, rhs: Self) -> Self { Self((self.0 << Self::FRAC_BITS) / rhs.0) }

    #[inline]
    pub fn to_i128_trunc(self) -> i128 { self.0 >> Self::FRAC_BITS }
}

impl fmt::Display for FixI128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Deterministic decimal with 6 fractional digits (truncated)
        let int = self.0 >> Self::FRAC_BITS;
        let frac = ((self.0 & ((1i128<<Self::FRAC_BITS)-1)) * 1_000_000) >> Self::FRAC_BITS;
        if self.0 < 0 {
            write!(f, "-{}.{:06}", (-int), (-frac))
        } else {
            write!(f, "{}.{:06}", int, frac)
        }
    }
}

/// Base dimensions: M,Kilogram,S,A,K,Mol,Cd (SI)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Dim { pub m:i8, pub kg:i8, pub s:i8, pub A:i8, pub K:i8, pub mol:i8, pub cd:i8 }

pub const DIM_ZERO: Dim = Dim{m:0,kg:0,s:0,A:0,K:0,mol:0,cd:0};

#[inline]
pub const fn add_dim(a: Dim, b: Dim) -> Dim {
    Dim{ m: a.m+b.m, kg: a.kg+b.kg, s:a.s+b.s, A:a.A+b.A, K:a.K+b.K, mol:a.mol+b.mol, cd:a.cd+b.cd }
}
#[inline]
pub const fn sub_dim(a: Dim, b: Dim) -> Dim {
    Dim{ m: a.m-b.m, kg: a.kg-b.kg, s:a.s-b.s, A:a.A-b.A, K:a.K-b.K, mol:a.mol-b.mol, cd:a.cd-b.cd }
}

/// A Unit is defined by its dimension and a fixed-point scale that converts FROM base unit TO this unit.
/// Example: gram: dim = kg^1, scale = 1000 (since 1 kg = 1000 g, base->g multiply by 1000).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Unit { pub dim: Dim, pub scale: FixI128 }

impl Unit {
    pub const fn new(dim: Dim, scale: FixI128) -> Self { Self { dim, scale } }
}

/// A Quantity is a fixed number tied to a Unit.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Quantity { pub val: FixI128, pub unit: Unit }

impl Quantity {
    /// Convert to target unit with same dimensions. Returns None if dimensions differ.
    pub fn convert_to(self, target: Unit) -> Option<Self> {
        if self.unit.dim != target.dim { return None; }
        // base_value = val / unit.scale
        let base_val = self.val.div(self.unit.scale);
        // target_val = base_value * target.scale
        Some(Self { val: base_val.mul(target.scale), unit: target })
    }
}

/// Standard unit definitions.
pub mod defs {
    use super::*;
    pub const M: Unit  = Unit{ dim: Dim{m:1,kg:0,s:0,A:0,K:0,mol:0,cd:0}, scale: FixI128::ONE };
    pub const KG: Unit = Unit{ dim: Dim{m:0,kg:1,s:0,A:0,K:0,mol:0,cd:0}, scale: FixI128::ONE };
    pub const S: Unit  = Unit{ dim: Dim{m:0,kg:0,s:1,A:0,K:0,mol:0,cd:0}, scale: FixI128::ONE };
    pub const MOL: Unit= Unit{ dim: Dim{m:0,kg:0,s:0,A:0,K:0,mol:1,cd:0}, scale: FixI128::ONE };

    // Derived dims
    pub const fn mul_dims(a: Dim, b: Dim) -> Dim { add_dim(a,b) }
    pub const fn pow_dim(a: Dim, p: i8) -> Dim {
        Dim{ m:a.m*p, kg:a.kg*p, s:a.s*p, A:a.A*p, K:a.K*p, mol:a.mol*p, cd:a.cd*p }
    }

    // Practical units
    /// Gram: 1 kg = 1000 g  => scale = 1000
    pub const G: Unit = Unit{ dim: KG.dim, scale: FixI128( (1000i128) << FixI128::FRAC_BITS ) };
    /// Milligram: 1 kg = 1_000_000 mg => scale = 1_000_000
    pub const MG: Unit = Unit{ dim: KG.dim, scale: FixI128( (1_000_000i128) << FixI128::FRAC_BITS ) };
    /// Meter^2
    pub const M2: Unit = Unit{ dim: mul_dims(M.dim, M.dim), scale: FixI128::ONE };
    /// Kilometer: 1 m -> 0.001 km => scale = 0.001; but we seldom use length directly
    pub const KM: Unit = Unit{ dim: M.dim, scale: FixI128::from_ratio(1, 1000) };
    /// km^2: from base m^2 to km^2 multiply by 1e-6
    pub const KM2: Unit = Unit{ dim: pow_dim(M.dim, 2), scale: FixI128::from_ratio(1, 1_000_000) };
    /// Per km^2 (m^-2). From base (per m^2) to per km^2 multiply by 1e6.
    pub const PER_KM2: Unit = Unit{ dim: pow_dim(M.dim, -2), scale: FixI128( (1_000_000i128) << FixI128::FRAC_BITS ) };

    /// Joule: kg*m^2/s^2 (base-derived) with scale 1.
    pub const J: Unit = Unit{ dim: add_dim(KG.dim, sub_dim(pow_dim(M.dim,2), pow_dim(S.dim,2))), scale: FixI128::ONE };
    /// Kilojoule: base J -> kJ multiply by 1/1000
    pub const KJ: Unit = Unit{ dim: J.dim, scale: FixI128::from_ratio(1, 1000) };
    /// Kilocalorie (thermochemical): 1 kcal = 4184 J => base J -> kcal multiply by 1/4184
    pub const KCAL: Unit = Unit{ dim: J.dim, scale: FixI128::from_ratio(1, 4184) };

    /// g/mol for atomic/molar masses: dim kg^1 * mol^-1, base->(g/mol) multiply by 1000
    pub const G_PER_MOL: Unit = Unit{ dim: add_dim(KG.dim, pow_dim(MOL.dim, -1)), scale: FixI128( (1000i128) << FixI128::FRAC_BITS ) };

    /// kg/m^3 density
    pub const KG_PER_M3: Unit = Unit{ dim: add_dim(KG.dim, pow_dim(M.dim, -3)), scale: FixI128::ONE };

    /// m/s (speed)
    pub const M_PER_S: Unit = Unit{ dim: add_dim(M.dim, pow_dim(S.dim, -1)), scale: FixI128::ONE };
}

/// Pretty (deterministic) formatting helpers
pub fn fmt_unit(u: &Unit) -> &'static str {
    use defs::*;
    // Minimal mapping, extend as needed
    if u.dim == KCAL.dim && u.scale.0 == KCAL.scale.0 { "kcal" }
    else if u.dim == defs::KJ.dim && u.scale.0 == defs::KJ.scale.0 { "kJ" }
    else if u.dim == defs::J.dim && u.scale.0 == defs::J.scale.0 { "J" }
    else if u.dim == defs::G.dim && u.scale.0 == defs::G.scale.0 { "g" }
    else if u.dim == defs::MG.dim && u.scale.0 == defs::MG.scale.0 { "mg" }
    else if u.dim == defs::KG.dim && u.scale.0 == defs::KG.scale.0 { "kg" }
    else if u.dim == defs::KM2.dim && u.scale.0 == defs::KM2.scale.0 { "km^2" }
    else if u.dim == defs::PER_KM2.dim && u.scale.0 == defs::PER_KM2.scale.0 { "1/km^2" }
    else if u.dim == defs::G_PER_MOL.dim && u.scale.0 == defs::G_PER_MOL.scale.0 { "g/mol" }
    else if u.dim == defs::KG_PER_M3.dim && u.scale.0 == defs::KG_PER_M3.scale.0 { "kg/m^3" }
    else if u.dim == defs::M_PER_S.dim && u.scale.0 == defs::M_PER_S.scale.0 { "m/s" }
    else { "<?>"} // extend mapping as needed
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.val, fmt_unit(&self.unit))
    }
}