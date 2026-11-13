use crate::app::preferences::Preferences;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum TranslationModel {
    #[default]
    Large,
    Small,
}

impl TranslationModel {
    pub const ALL: [TranslationModel; 2] = [TranslationModel::Large, TranslationModel::Small];

    pub fn selected() -> Self {
        Preferences::load().translation_model
    }

    pub fn id(self) -> &'static str {
        match self {
            TranslationModel::Large => "hunyuan-mt-7b",
            TranslationModel::Small => "hy-mt1.5-1.8b",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|model| model.id() == id)
    }

    pub fn display_name(self) -> &'static str {
        match self {
            TranslationModel::Large => "Hunyuan-MT-7B",
            TranslationModel::Small => "HY-MT1.5-1.8B",
        }
    }

    pub fn download_size(self) -> String {
        match self {
            TranslationModel::Large => t!("size.model_large").to_string(),
            TranslationModel::Small => t!("size.model_small").to_string(),
        }
    }

    pub fn source(self) -> &'static str {
        match self {
            TranslationModel::Large => "hf.co/mradermacher/Hunyuan-MT-7B-GGUF:Q4_K_M",
            TranslationModel::Small => "hf.co/tencent/HY-MT1.5-1.8B-GGUF:Q4_K_M",
        }
    }

    pub fn ollama_name(self) -> &'static str {
        match self {
            TranslationModel::Large => self.source(),
            TranslationModel::Small => "kannichan/hy-mt1.5-1.8b",
        }
    }

    pub fn fixed_template(self) -> Option<(&'static str, &'static str)> {
        match self {
            TranslationModel::Large => None,

            TranslationModel::Small => Some((
                "<｜hy_begin▁of▁sentence｜>{{ if .System }}{{ .System }}<｜hy_place▁holder▁no▁3｜>{{ end }}\
                 <｜hy_User｜>{{ .Prompt }}<｜hy_Assistant｜>{{ .Response }}",
                "<｜hy_place▁holder▁no▁2｜>",
            )),
        }
    }
}
