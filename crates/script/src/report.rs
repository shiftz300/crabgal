use crabgal_core::{Action, ChoiceTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub span: SourceSpan,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Background,
    Figure,
    Voice,
    Bgm,
    Effect,
    MiniAvatar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRef {
    pub path: String,
    pub kind: ResourceKind,
    pub action_index: usize,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneRef {
    pub scene: String,
    pub action_index: usize,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Default)]
pub struct ParseReport {
    pub actions: Vec<Action>,
    pub spans: Vec<SourceSpan>,
    pub diagnostics: Vec<Diagnostic>,
    pub resources: Vec<ResourceRef>,
    pub sub_scenes: Vec<SceneRef>,
}

impl ParseReport {
    pub(crate) fn push(&mut self, action: Action, span: SourceSpan) {
        let action_index = self.actions.len();
        collect_references(&action, action_index, span, self);
        self.actions.push(action);
        self.spans.push(span);
    }
}

fn collect_references(
    action: &Action,
    action_index: usize,
    span: SourceSpan,
    report: &mut ParseReport,
) {
    let mut resource = |path: &str, kind| {
        if !path.is_empty() && path != "none" {
            report.resources.push(ResourceRef {
                path: path.to_owned(),
                kind,
                action_index,
                span,
            });
        }
    };
    match action {
        Action::ShowBg { image, .. } => resource(image, ResourceKind::Background),
        Action::ShowSprite { image, .. } => resource(image, ResourceKind::Figure),
        Action::Say { options, .. } => {
            if let Some(vocal) = &options.vocal {
                resource(vocal, ResourceKind::Voice);
            }
        }
        Action::Bgm { file, .. } => resource(file, ResourceKind::Bgm),
        Action::Effect {
            file: Some(file), ..
        } => resource(file, ResourceKind::Effect),
        Action::MiniAvatar { image } => resource(image, ResourceKind::MiniAvatar),
        Action::ChangeScene(scene) | Action::CallScene(scene) => {
            report.sub_scenes.push(SceneRef {
                scene: scene.clone(),
                action_index,
                span,
            });
        }
        Action::Menu { choices, .. } => {
            for choice in choices {
                if let ChoiceTarget::ChangeScene(scene) | ChoiceTarget::CallScene(scene) =
                    &choice.target
                {
                    report.sub_scenes.push(SceneRef {
                        scene: scene.clone(),
                        action_index,
                        span,
                    });
                }
            }
        }
        Action::Flow { action, .. } => {
            collect_references(action, action_index, span, report);
        }
        _ => {}
    }
}
