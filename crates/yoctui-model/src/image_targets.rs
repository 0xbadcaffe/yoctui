//! Shared image-target projection for the picker, menus and operator guide.
use crate::{App, Recipe};

fn image_recipe_candidate(recipe: &Recipe) -> bool {
    let name = recipe.name.as_str();
    if name.starts_with("nativesdk-") || name.ends_with("-native") {
        return false;
    }
    name.starts_with("image-")
        || name.ends_with("-image")
        || name.contains("-image-")
        || recipe
            .file
            .as_deref()
            .and_then(std::path::Path::parent)
            .and_then(std::path::Path::file_name)
            .is_some_and(|directory| directory == "images")
}

impl App {
    pub fn has_image_targets(&self) -> bool {
        self.build_environment.connected()
            && self.workspace.recipes.iter().any(image_recipe_candidate)
    }

    pub fn is_image_target(&self, target: &str) -> bool {
        self.build_environment.connected()
            && self
                .workspace
                .recipes
                .iter()
                .any(|recipe| recipe.name == target && image_recipe_candidate(recipe))
    }

    /// Recipe summaries do not carry inherited classes. Use provider-directory
    /// and image-token candidates, not substring matches such as python-imagesize.
    /// BitBake still validates the reviewed build target.
    pub fn image_targets(&self) -> Vec<String> {
        if !self.build_environment.connected() {
            return Vec::new();
        }
        let mut images = self
            .workspace
            .recipes
            .iter()
            .filter(|recipe| image_recipe_candidate(recipe))
            .map(|recipe| recipe.name.clone())
            .collect::<Vec<_>>();
        images.sort();
        images.dedup();
        images
    }

    pub fn refresh_image_targets(&mut self) {
        self.available_images = self.image_targets();
    }
}
