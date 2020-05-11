use crate::ID;

pub const MAX_ORDERED: Ordered = Ordered(std::u32::MAX);
pub const MIN_ORDERED: Ordered = Ordered(std::u32::MIN);

/// ChildOf is the source of truth when it comes to the structure of things in trees.
///
/// .0 is parent ID, .1 is Ordered relative to siblings
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChildOf(pub ID, pub Ordered);

impl ChildOf {
    pub fn new(child_of: ID, hint: u8) -> Self {
        ChildOf(child_of, Ordered::hinted(hint))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ordered(u32);

impl Ordered {
    /// Create an ordered component with a hint of what it's initial order should be
    pub fn hinted(hint: u8) -> Self {
        const EIGHTH_MAX: u32 = std::u32::MAX / 8;
        Ordered(((hint as u32).pow(3) + (hint as u32) * 4) + EIGHTH_MAX)
    }

    /// Mutate version of "between"
    pub fn move_between(&mut self, min: &Self, max: &Self) {
        self.0 = (min.0 / 2) + (max.0 / 2);
    }

    // 👇 Somewhat thought through ordering logic inspired by fractional indexing

    pub fn between(min: &Self, max: &Self) -> Self {
        Ordered((min.0 / 2) + (max.0 / 2))
    }

    pub fn after(a: &Self) -> Self {
        const HALF_MAX: u32 = std::u32::MAX / 2;
        // average between a and max
        Ordered((a.0 / 2) + HALF_MAX)
    }

    pub fn before(a: &Self) -> Self {
        // I know this is zero, but for posterity let's think about this conceptually as the average between a & min.
        const HALF_MIN: u32 = std::u32::MIN / 2;
        Ordered((a.0 / 2) + HALF_MIN)
    }
}
