use eframe::egui::{self, Id};
use epaint::{Color32, TextureHandle};
use truncate_core::board::Coordinate;

use crate::{
    regions::active_game::GameCtx,
    utils::{mapper::MappedTile, Darken, Lighten, Theme},
};

use super::{character::CharacterOrient, CharacterUI};

pub enum TilePlayer {
    Own,
    Enemy(usize),
}

pub struct TileUI {
    letter: char,
    player: TilePlayer,
    selected: bool,
    highlighted: bool,
    active: bool,
    hovered: bool,
    ghost: bool,
    added: bool,
    modified: bool,
    defeated: bool,
    truncated: bool,
    won: bool,
    id: Option<Id>,
}

impl TileUI {
    pub fn new(letter: char, player: TilePlayer) -> Self {
        Self {
            letter,
            player,
            selected: false,
            highlighted: false,
            active: true,
            hovered: false,
            ghost: false,
            added: false,
            modified: false,
            defeated: false,
            truncated: false,
            won: false,
            id: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn hovered(mut self, hovered: bool) -> Self {
        self.hovered = hovered;
        self
    }

    pub fn ghost(mut self, ghost: bool) -> Self {
        self.ghost = ghost;
        self
    }

    pub fn added(mut self, added: bool) -> Self {
        self.added = added;
        self
    }

    pub fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    pub fn defeated(mut self, defeated: bool) -> Self {
        self.defeated = defeated;
        self
    }

    pub fn truncated(mut self, truncated: bool) -> Self {
        self.truncated = truncated;
        self
    }

    pub fn won(mut self, won: bool) -> Self {
        self.won = won;
        self
    }

    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }
}

impl TileUI {
    fn tile_color(&self, hovered: bool, theme: &Theme, ctx: &GameCtx) -> Color32 {
        if self.highlighted && ctx.current_time.subsec_millis() > 500 {
            theme.selection.pastel()
        } else if self.won || self.selected {
            theme.selection
        } else if self.defeated || self.truncated || !self.active {
            theme.text
        } else {
            match (&self.player, hovered) {
                (TilePlayer::Own, false) => ctx.player_colors[ctx.player_number as usize].pastel(),
                (TilePlayer::Own, true) => ctx.player_colors[ctx.player_number as usize]
                    .pastel()
                    .lighten(),
                (TilePlayer::Enemy(p), false) => ctx.player_colors[*p].pastel(),
                (TilePlayer::Enemy(p), true) => ctx.player_colors[*p].pastel().lighten(),
            }
        }
    }

    pub fn render(
        self,
        coord: Option<Coordinate>,
        ui: &mut egui::Ui,
        ctx: &mut GameCtx,
        capture_clicks: bool,
        rescale: Option<f32>,
    ) -> egui::Response {
        let theme = rescale
            .map(|v| ctx.theme.rescale(v))
            .unwrap_or_else(|| ctx.theme.clone());

        // TODO: Remove magic number somehow (currently 2px/16px for tile sprite border)
        let tile_margin = theme.grid_size * 0.125;

        let (mut base_rect, _) = ui.allocate_exact_size(
            egui::vec2(theme.grid_size, theme.grid_size),
            egui::Sense::hover(),
        );

        let mut tile_rect = base_rect.shrink(tile_margin);
        let tile_sense = if capture_clicks {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let mut response = ui.allocate_rect(tile_rect, tile_sense);

        if let Some(id) = self.id {
            response = ui.interact(tile_rect, id, egui::Sense::click_and_drag());
        }

        let hovered = (response.hovered() || self.hovered) && (!self.truncated && !self.defeated);
        if hovered {
            if !self.ghost {
                base_rect = base_rect.translate(egui::vec2(0.0, tile_margin * -1.0));
                tile_rect = tile_rect.translate(egui::vec2(0.0, tile_margin * -1.0));
            }
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
        }

        if ui.is_rect_visible(base_rect) {
            let outline = if self.added {
                Some(theme.selection)
            } else if self.modified {
                Some(theme.modification)
            } else {
                None
            };

            let tile_color = self.tile_color(hovered, &theme, ctx);
            let mapped_tile = if self.ghost {
                MappedTile::new(None, Some(tile_color), coord, ctx.map_texture.clone())
            } else {
                MappedTile::new(Some(tile_color), outline, coord, ctx.map_texture.clone())
            };
            mapped_tile.render(base_rect, ui);

            let mut char_rect = tile_rect.clone();
            char_rect.set_height(char_rect.height() - tile_margin * 0.5);

            CharacterUI::new(
                self.letter,
                match self.player {
                    TilePlayer::Own => CharacterOrient::North,
                    TilePlayer::Enemy(_) => CharacterOrient::South,
                },
            )
            .hovered(hovered)
            .selected(self.selected)
            .active(self.active)
            .ghost(self.ghost)
            .defeated(self.defeated)
            .truncated(self.truncated)
            .render(ui, char_rect, &theme);
        }

        // let outline = if self.selected {
        //     Some(theme.selection)
        // } else if self.added {
        //     Some(theme.addition)
        // } else if self.modified {
        //     Some(theme.modification)
        // } else {
        //     None
        // };

        // if let Some(outline) = outline {
        //     ui.painter().rect_stroke(
        //         tile_rect.expand(theme.tile_margin * 0.5),
        //         theme.rounding * 1.3,
        //         Stroke::new(theme.tile_margin, outline),
        //     )
        // }

        response
    }
}