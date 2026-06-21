use crate::ports::font::FontValidatorPort;
use cosmic_text::FontSystem;

pub struct CosmicFontValidatorAdapter {
    font_system: FontSystem,
}

impl CosmicFontValidatorAdapter {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
        }
    }
}

impl FontValidatorPort for CosmicFontValidatorAdapter {
    fn is_valid_family(&self, family: &str) -> bool {
        if family.is_empty() {
            return true;
        }

        // cosmic-text uses its own Family enum for matches,
        // but we can query the internal database for exact family name matches.
        self.font_system.db().faces().any(|face| {
            face.families
                .iter()
                .any(|(f, _)| f.to_lowercase() == family.to_lowercase())
        })
    }
}

impl Default for CosmicFontValidatorAdapter {
    fn default() -> Self {
        Self::new()
    }
}
