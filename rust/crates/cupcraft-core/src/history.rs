//! Historial de deshacer/rehacer.
//!
//! La app JS guarda cadenas JSON del proyecto (tope 80 entradas). Aquí se
//! hace lo mismo: cada mutación empuja el estado anterior y `undo`/`redo`
//! intercambian entre las dos pilas.

use crate::model::Project;

/// Tope de pasos de deshacer, igual que en `store.js`.
pub const HISTORY_LIMIT: usize = 80;

#[derive(Debug, Default)]
pub struct History {
    past: Vec<String>,
    future: Vec<String>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    /// Guarda el estado previo a una mutación y limpia el futuro.
    pub fn push(&mut self, before: &Project) {
        self.past.push(serde_json::to_string(before).unwrap_or_default());
        if self.past.len() > HISTORY_LIMIT {
            self.past.remove(0);
        }
        self.future.clear();
    }

    /// Deshace: devuelve el proyecto anterior y deja el actual para rehacer.
    pub fn undo(&mut self, current: &Project) -> Option<Project> {
        let previous = self.past.pop()?;
        self.future.push(serde_json::to_string(current).unwrap_or_default());
        serde_json::from_str(&previous).ok()
    }

    /// Rehace: devuelve el proyecto siguiente y lo guarda como actual.
    pub fn redo(&mut self, current: &Project) -> Option<Project> {
        let next = self.future.pop()?;
        self.past.push(serde_json::to_string(current).unwrap_or_default());
        serde_json::from_str(&next).ok()
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    pub fn clear(&mut self) {
        self.past.clear();
        self.future.clear();
    }

    pub fn len(&self) -> usize {
        self.past.len()
    }

    pub fn is_empty(&self) -> bool {
        self.past.is_empty()
    }
}

/// Envoltorio que mantiene proyecto + historial sincronizados.
#[derive(Debug, Default)]
pub struct Editor {
    project: Option<Project>,
    history: History,
    selection: Option<String>,
}

impl Editor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, project: Project) {
        self.project = Some(project);
        self.history.clear();
        self.selection = None;
    }

    pub fn project(&self) -> Option<&Project> {
        self.project.as_ref()
    }

    pub fn project_mut(&mut self) -> Option<&mut Project> {
        self.project.as_mut()
    }

    pub fn take_project(&mut self) -> Option<Project> {
        self.project.take()
    }

    pub fn selection(&self) -> Option<&str> {
        self.selection.as_deref()
    }

    pub fn select(&mut self, clip_id: Option<String>) {
        self.selection = clip_id;
    }

    /// Aplica una mutación registrándola en el historial.
    pub fn mutate<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Project) -> R,
    {
        let project = self.project.as_mut()?;
        let before = project.clone();
        let result = f(project);
        project.updated_at = crate::model::now_millis();
        self.history.push(&before);
        // Si la selección apuntaba a un clip que ya no existe, se limpia.
        if let Some(sel) = &self.selection {
            if project.find_clip(sel).is_none() {
                self.selection = None;
            }
        }
        Some(result)
    }

    /// Mutación sin registro (p. ej. carga o corrección interna).
    pub fn mutate_silent<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Project) -> R,
    {
        let project = self.project.as_mut()?;
        Some(f(project))
    }

    pub fn undo(&mut self) -> bool {
        let Some(current) = self.project.take() else { return false };
        match self.history.undo(&current) {
            Some(previous) => {
                if let Some(sel) = &self.selection {
                    if previous.find_clip(sel).is_none() {
                        self.selection = None;
                    }
                }
                self.project = Some(previous);
                true
            }
            None => {
                self.project = Some(current);
                false
            }
        }
    }

    pub fn redo(&mut self) -> bool {
        let Some(current) = self.project.take() else { return false };
        match self.history.redo(&current) {
            Some(next) => {
                if let Some(sel) = &self.selection {
                    if next.find_clip(sel).is_none() {
                        self.selection = None;
                    }
                }
                self.project = Some(next);
                true
            }
            None => {
                self.project = Some(current);
                false
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ClipKind, Project};

    fn editor_with_clip() -> Editor {
        let mut ed = Editor::new();
        ed.open(Project::new("Demo"));
        ed.mutate(|p| {
            p.add_media_clip(None, ClipKind::Image, "foto.png", 5.0, None, None);
        });
        ed
    }

    #[test]
    fn undo_restores_previous_state() {
        let mut ed = editor_with_clip();
        assert_eq!(ed.project().unwrap().tracks[1].clips.len(), 1);
        assert!(ed.undo());
        assert_eq!(ed.project().unwrap().tracks[1].clips.len(), 0);
        assert!(ed.can_redo());
    }

    #[test]
    fn redo_reapplies() {
        let mut ed = editor_with_clip();
        ed.undo();
        assert!(ed.redo());
        assert_eq!(ed.project().unwrap().tracks[1].clips.len(), 1);
        assert!(!ed.can_redo());
    }

    #[test]
    fn new_mutation_clears_the_future() {
        let mut ed = editor_with_clip();
        ed.undo();
        ed.mutate(|p| p.add_track(crate::model::TrackType::Video));
        assert!(!ed.can_redo());
    }

    #[test]
    fn history_is_capped_at_80() {
        let mut ed = Editor::new();
        ed.open(Project::new("Demo"));
        for _ in 0..120 {
            ed.mutate(|p| p.add_track(crate::model::TrackType::Video));
        }
        assert_eq!(ed.history.len(), HISTORY_LIMIT);
    }

    #[test]
    fn selection_clears_when_the_clip_disappears() {
        let mut ed = editor_with_clip();
        let id = ed.project().unwrap().tracks[1].clips[0].id.clone();
        ed.select(Some(id));
        ed.undo(); // el clip desaparece
        assert!(ed.selection().is_none());
    }

    #[test]
    fn undo_without_history_is_a_noop() {
        let mut ed = Editor::new();
        ed.open(Project::new("Demo"));
        assert!(!ed.undo());
        assert!(ed.project().is_some());
    }
}
