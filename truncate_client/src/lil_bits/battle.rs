use eframe::{
    egui::{self, FontId, Layout, RichText, Sense},
    emath::Align,
};
use epaint::{vec2, Color32};
use truncate_core::reporting::{BattleReport, BattleWord};

use crate::regions::active_game::GameCtx;

pub struct BattleUI<'a> {
    battle: &'a BattleReport,
}

impl<'a> BattleUI<'a> {
    pub fn new(battle: &'a BattleReport) -> Self {
        Self { battle }
    }
}

fn render_word(battle_word: &BattleWord, ctx: &GameCtx, ui: &mut egui::Ui) {
    let galley = ui.painter().layout_no_wrap(
        battle_word.word.clone(),
        FontId::new(
            ctx.theme.letter_size,
            egui::FontFamily::Name("Truncate-Heavy".into()),
        ),
        match battle_word.valid {
            Some(true) => ctx.theme.addition,
            Some(false) => ctx.theme.defeated,
            None => ctx.theme.outlines,
        },
    );
    let (rect, resp) = ui.allocate_at_least(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley);

    resp.on_hover_ui(|ui| match (battle_word.valid, &battle_word.meanings) {
        (None, _) => {
            ui.label("Word did not need to be checked as the attack was invalid");
        }
        (Some(true), None) => {
            ui.label("Valid word with no definition found");
        }
        (Some(true), Some(meanings)) => {
            for meaning in meanings {
                ui.label(format!("{}:", meaning.pos));
                for def in &meaning.defs {
                    ui.label(format!("  • {def}"));
                }
            }
        }
        (Some(false), _) => {
            ui.label("Invalid word");
        }
    });
}

impl<'a> BattleUI<'a> {
    pub fn render(self, ctx: &GameCtx, ui: &mut egui::Ui) {
        let mut theme = ctx.theme.rescale(0.5);
        theme.tile_margin = 0.0;

        ui.allocate_ui_with_layout(
            vec2(ui.available_size_before_wrap().x, 0.0),
            Layout::left_to_right(Align::BOTTOM).with_main_wrap(true),
            |ui| {
                for battle_word in &self.battle.attackers {
                    render_word(battle_word, ctx, ui);
                }

                match self.battle.outcome {
                    truncate_core::judge::Outcome::AttackerWins(_) => {
                        ui.label(RichText::new("Beats").color(Color32::WHITE));
                    }
                    truncate_core::judge::Outcome::DefenderWins => {
                        ui.label(RichText::new("Loses to").color(Color32::WHITE));
                    }
                }

                for battle_word in &self.battle.defenders {
                    render_word(battle_word, ctx, ui);
                }
            },
        );
    }
}