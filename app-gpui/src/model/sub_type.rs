/// Mirrors `utils::SubType`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubType {
    pub rank: Option<i64>,
    pub charges: Option<i64>,
    pub variant: Option<String>,
    pub amber_stars: Option<i64>,
    pub cyan_stars: Option<i64>,
}

impl SubType {
    pub fn rank(rank: i64) -> Self {
        Self {
            rank: Some(rank),
            ..Self::default()
        }
    }

    pub fn variant(variant: &str) -> Self {
        Self {
            variant: Some(variant.to_owned()),
            ..Self::default()
        }
    }

    pub fn stars(amber_stars: i64, cyan_stars: i64) -> Self {
        Self {
            amber_stars: Some(amber_stars),
            cyan_stars: Some(cyan_stars),
            ..Self::default()
        }
    }

    /// The text of the rank column: a mod or arcane rank, a relic refinement,
    /// or the socketed stars of an Ayatan sculpture.
    pub fn label(&self) -> Option<String> {
        if let Some(rank) = self.rank {
            return Some(format!("Rank {rank}"));
        }
        if let Some(variant) = &self.variant {
            let mut chars = variant.chars();
            return chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect());
        }
        if let (Some(amber), Some(cyan)) = (self.amber_stars, self.cyan_stars) {
            return Some(format!("{amber} amber, {cyan} cyan"));
        }
        self.charges.map(|charges| format!("{charges} charges"))
    }
}
