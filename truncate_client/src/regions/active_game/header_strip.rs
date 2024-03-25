use epaint::{emath::Align2, hex_color, vec2, Color32, FontId, Rect, TextureHandle, Vec2};
use instant::Duration;
use truncate_core::{
    board::{Board, Coordinate},
    generation::BoardSeed,
    messages::{GamePlayerMessage, GameStateMessage, PlayerMessage, RoomCode},
    npc::scoring::{NPCParams, NPCPersonality},
    player::Hand,
    reporting::{BoardChange, BoardChangeAction, BoardChangeDetail, Change, TimeChange},
};

use eframe::{
    egui::{self, CursorIcon, Layout, Order, ScrollArea, Sense},
    emath::Align,
};
use hashbrown::HashMap;

use crate::{
    lil_bits::{BattleUI, BoardUI, DictionaryUI, HandUI, TimerUI},
    utils::{
        depot::{
            AestheticDepot, AudioDepot, BoardDepot, GameplayDepot, InteractionDepot, RegionDepot,
            TimingDepot, TruncateDepot, UIStateDepot,
        },
        macros::tr_log,
        mapper::{MappedBoard, MappedTiles},
        tex::{render_tex_quad, render_tex_quads, tiles},
        text::TextHelper,
        timing::get_qs_tick,
        urls::back_to_menu,
        Lighten, Theme,
    },
};

use super::{ActiveGame, HeaderType};

impl ActiveGame {
    pub fn render_header_strip(
        &mut self,
        ui: &mut egui::Ui,
        game_ref: Option<&truncate_core::game::Game>,
    ) -> (Option<Rect>, Option<PlayerMessage>) {
        if matches!(self.depot.ui_state.game_header, HeaderType::None) {
            return (None, None);
        }

        let timer_area = ui.available_rect_before_wrap();
        let avail_width = ui.available_width();
        let mut msg = None;

        let area = egui::Area::new(egui::Id::new("timers_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::LEFT_TOP, vec2(timer_area.left(), timer_area.top()));

        let resp = area.show(ui.ctx(), |ui| {
            // TODO: We can likely use Memory::area_rect now instead of tracking sizes ourselves
            if let Some(bg_rect) = self.depot.regions.headers_total_rect {
                ui.painter().clone().rect_filled(
                    bg_rect,
                    0.0,
                    self.depot.aesthetics.theme.water.gamma_multiply(0.9),
                );
            }

            ui.add_space(5.0);

            ui.allocate_ui_with_layout(
                vec2(avail_width, 10.0),
                Layout::right_to_left(Align::TOP),
                |ui| {
                    ui.expand_to_include_x(timer_area.left());
                    ui.expand_to_include_x(timer_area.right());

                    ui.spacing_mut().item_spacing = Vec2::splat(0.0);
                    let item_spacing = 10.0;
                    let button_size = 48.0;

                    if !self.depot.ui_state.is_mobile {
                        ui.add_space(item_spacing);
                        let (mut sidebar_button_rect, sidebar_button_resp) =
                            ui.allocate_exact_size(Vec2::splat(button_size), Sense::click());
                        if sidebar_button_resp.hovered() {
                            sidebar_button_rect = sidebar_button_rect.translate(vec2(0.0, -2.0));
                            ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                        }

                        if !self.depot.ui_state.sidebar_toggled {
                            if self.depot.ui_state.unread_sidebar {
                                render_tex_quads(
                                    &[tiles::quad::INFO_BUTTON, tiles::quad::BUTTON_NOTIFICATION],
                                    sidebar_button_rect,
                                    &self.depot.aesthetics.map_texture,
                                    ui,
                                );
                            } else {
                                render_tex_quad(
                                    tiles::quad::INFO_BUTTON,
                                    sidebar_button_rect,
                                    &self.depot.aesthetics.map_texture,
                                    ui,
                                );
                            }
                        } else {
                            render_tex_quad(
                                tiles::quad::TRI_EAST_BUTTON,
                                sidebar_button_rect,
                                &self.depot.aesthetics.map_texture,
                                ui,
                            );
                        }

                        if sidebar_button_resp.clicked() {
                            self.depot.ui_state.sidebar_toggled =
                                !self.depot.ui_state.sidebar_toggled;
                            self.depot.ui_state.unread_sidebar = false;
                        }

                        ui.add_space(item_spacing);
                    }

                    let remaining_width = ui.available_width();
                    let total_width = 700.0_f32.min(remaining_width);
                    let padding = (remaining_width - total_width) / 2.0;

                    ui.add_space(padding);

                    match &self.depot.ui_state.game_header {
                        HeaderType::Timers => {
                            ui.add_space(item_spacing);

                            let timer_width = (total_width - item_spacing * 3.0) / 2.0;

                            if let Some(player) = self
                                .players
                                .iter()
                                .find(|p| p.index == self.depot.gameplay.player_number as usize)
                            {
                                TimerUI::new(player, &self.depot, &self.time_changes)
                                    .friend(true)
                                    .active(
                                        player.index
                                            == self.depot.gameplay.next_player_number as usize,
                                    )
                                    .render(Some(timer_width), false, ui);
                            }

                            ui.add_space(item_spacing);

                            if let Some(opponent) = self
                                .players
                                .iter()
                                .find(|p| p.index != self.depot.gameplay.player_number as usize)
                            {
                                TimerUI::new(opponent, &self.depot, &self.time_changes)
                                    .friend(false)
                                    .active(
                                        opponent.index
                                            == self.depot.gameplay.next_player_number as usize,
                                    )
                                    .right_align()
                                    .render(Some(timer_width), false, ui);
                            }

                            ui.add_space(item_spacing);
                        }
                        HeaderType::Summary { title, attempt } => {
                            let summary_height = 50.0;
                            let summary_width = ui.available_width();
                            let (rect, _) = ui.allocate_exact_size(
                                vec2(summary_width, summary_height),
                                Sense::hover(),
                            );
                            let mut ui = ui.child_ui(rect, Layout::top_down(Align::LEFT));

                            let active_player = self.depot.gameplay.player_number;
                            let summary = if let Some(game) = game_ref {
                                format!(
                                    "{} move{}",
                                    game.player_turn_count[active_player as usize],
                                    if game.player_turn_count[active_player as usize] == 1 {
                                        ""
                                    } else {
                                        "s"
                                    },
                                )
                            } else {
                                "".to_string()
                            };

                            let mut fz = 14.0;
                            let mut title_text = TextHelper::heavy(title, fz, None, &mut ui);
                            while title_text.mesh_size().x > summary_width {
                                fz -= 1.0;
                                title_text = TextHelper::heavy(title, fz, None, &mut ui)
                            }
                            let title_text_mesh_size = title_text.mesh_size();
                            let title_x_offset = (summary_width - title_text_mesh_size.x) / 2.0;

                            let mut fz = 10.0;
                            let mut summary_text = TextHelper::heavy(&summary, fz, None, &mut ui);
                            while summary_text.mesh_size().x > summary_width {
                                fz -= 1.0;
                                summary_text = TextHelper::heavy(&summary, fz, None, &mut ui);
                            }

                            let summary_text_mesh_size = summary_text.mesh_size();
                            let summary_x_offset = (summary_width - summary_text_mesh_size.x) / 2.0;

                            let spacing = 5.0;
                            let y_offset = (summary_height
                                - summary_text_mesh_size.y
                                - title_text_mesh_size.y)
                                / 2.0;
                            ui.add_space(y_offset);

                            let (rect, _) = ui.allocate_exact_size(
                                vec2(ui.available_width(), title_text_mesh_size.y),
                                Sense::hover(),
                            );
                            title_text.paint_at(
                                rect.min + vec2(title_x_offset, 0.0),
                                self.depot.aesthetics.theme.text,
                                &mut ui,
                            );
                            ui.add_space(spacing);

                            let (rect, _) = ui.allocate_exact_size(
                                vec2(ui.available_width(), summary_text_mesh_size.y),
                                Sense::hover(),
                            );
                            summary_text.paint_at(
                                rect.min + vec2(summary_x_offset, 0.0),
                                self.depot.aesthetics.theme.text,
                                &mut ui,
                            );

                            ui.add_space(y_offset);
                        }
                        HeaderType::None => unreachable!(),
                    }

                    ui.add_space(item_spacing);
                },
            );

            ui.add_space(10.0);
        });

        self.depot.regions.headers_total_rect = Some(resp.response.rect);

        (Some(resp.response.rect), msg)
    }
}
