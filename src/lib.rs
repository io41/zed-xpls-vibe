use zed_extension_api as zed;

struct UpXplsExtension;

impl zed::Extension for UpXplsExtension {
    fn new() -> Self {
        Self
    }
}

zed::register_extension!(UpXplsExtension);
