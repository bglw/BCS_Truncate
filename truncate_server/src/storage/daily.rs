use jwt_simple::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use truncate_core::{
    messages::{DailyStateMessage, TruncateToken},
    moves::{self, packing::pack_moves, Move},
};
use uuid::Uuid;

use crate::{errors::TruncateServerError, ServerState};

use super::accounts::AuthedTruncateToken;

pub struct AttemptRecord {
    attempt_id: Uuid,
    attempt_number: i32,
    sequence_of_moves: String,
    won: bool,
}
pub struct DailyPuzzleRecord {
    result_id: Uuid,
}

/// Returns any partial or completed attempt for a given player on the requested day.
pub async fn load_attempt(
    server_state: &ServerState,
    player: AuthedTruncateToken,
    daily_puzzle: i32,
) -> Result<Option<DailyStateMessage>, TruncateServerError> {
    let Some(daily_puzzle_record) = get_day_record(server_state, player, daily_puzzle).await?
    else {
        return Ok(None);
    };

    let Some(attempt_record) =
        get_latest_attempt_for_day(server_state, daily_puzzle_record.result_id).await?
    else {
        return Ok(None);
    };

    let Ok(current_moves) = moves::packing::unpack_moves(&attempt_record.sequence_of_moves, 2)
    else {
        // If move parsing fails, move on as if there was no attempt.
        return Ok(None);
    };

    Ok(Some(DailyStateMessage {
        puzzle_day: daily_puzzle.try_into().unwrap_or_default(),
        attempt: attempt_record.attempt_number.try_into().unwrap_or_default(),
        current_moves,
    }))
}

pub async fn get_or_create_latest_attempt(
    server_state: &ServerState,
    player: AuthedTruncateToken,
    daily_puzzle: i32,
    human_player: i32,
) -> Result<(DailyStateMessage, AttemptRecord), TruncateServerError> {
    let Some(pool) = &server_state.truncate_db else {
        return Err(TruncateServerError::DatabaseOffline);
    };
    let player_id = player.player();

    let daily_puzzle_record = get_day_record(server_state, player, daily_puzzle).await?;

    let result_id = if let Some(dpr) = daily_puzzle_record {
        dpr.result_id
    } else {
        let new_puzzle_record = sqlx::query_as!(
            DailyPuzzleRecord,
            "INSERT INTO daily_puzzle_results (player_id, daily_puzzle, human_player) VALUES ($1, $2, $3) RETURNING result_id",
            player_id,
            daily_puzzle,
            human_player
        )
        .fetch_one(pool)
        .await?;

        new_puzzle_record.result_id
    };

    let mut latest_attempt = match get_latest_attempt_for_day(server_state, result_id).await? {
        Some(attempt) => attempt,
        None => create_new_attempt(server_state, result_id).await?,
    };

    let current_moves = match moves::packing::unpack_moves(&latest_attempt.sequence_of_moves, 2) {
        Ok(moves) => moves,
        Err(_) => {
            // Something went wrong with this attempt — move on to a new one.
            latest_attempt = create_new_attempt(server_state, result_id).await?;
            vec![]
        }
    };

    Ok((
        DailyStateMessage {
            puzzle_day: daily_puzzle.try_into().unwrap_or_default(),
            attempt: latest_attempt.attempt_number.try_into().unwrap_or_default(),
            current_moves,
        },
        latest_attempt,
    ))
}

async fn get_day_record(
    server_state: &ServerState,
    player: AuthedTruncateToken,
    daily_puzzle: i32,
) -> Result<Option<DailyPuzzleRecord>, TruncateServerError> {
    let Some(pool) = &server_state.truncate_db else {
        return Err(TruncateServerError::DatabaseOffline);
    };
    let player_id = player.player();

    let daily_puzzle_record = sqlx::query_as!(
        DailyPuzzleRecord,
        "SELECT result_id FROM daily_puzzle_results WHERE player_id = $1 AND daily_puzzle = $2",
        player_id,
        daily_puzzle
    )
    .fetch_optional(pool)
    .await?;

    Ok(daily_puzzle_record)
}

async fn get_latest_attempt_for_day(
    server_state: &ServerState,
    result_id: Uuid,
) -> Result<Option<AttemptRecord>, TruncateServerError> {
    let Some(pool) = &server_state.truncate_db else {
        return Err(TruncateServerError::DatabaseOffline);
    };

    sqlx::query_as!(
        AttemptRecord,
        "SELECT attempt_id, sequence_of_moves, attempt_number, won FROM daily_puzzle_attempts WHERE result_id = $1 ORDER BY attempt_number DESC LIMIT 1",
        result_id
    )
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

async fn create_new_attempt(
    server_state: &ServerState,
    result_id: Uuid,
) -> Result<AttemptRecord, TruncateServerError> {
    let Some(pool) = &server_state.truncate_db else {
        return Err(TruncateServerError::DatabaseOffline);
    };

    let latest_attempt = get_latest_attempt_for_day(server_state, result_id).await?;
    let new_attempt_number = latest_attempt
        .map(|a| a.attempt_number + 1)
        .unwrap_or_default();

    let new_attempt = sqlx::query!(
        "INSERT INTO daily_puzzle_attempts (result_id, attempt_number) VALUES ($1, $2) RETURNING attempt_id",
        result_id,
        new_attempt_number
    )
    .fetch_one(pool)
    .await?;

    // Return the new attempt information
    Ok(AttemptRecord {
        attempt_id: new_attempt.attempt_id,
        attempt_number: new_attempt_number,
        sequence_of_moves: String::new(),
        won: false,
    })
}

pub async fn persist_moves(
    server_state: &ServerState,
    player: AuthedTruncateToken,
    daily_puzzle: i32,
    human_player: i32,
    moves: Vec<Move>,
    won: bool,
) -> Result<(), TruncateServerError> {
    let Some(pool) = &server_state.truncate_db else {
        return Err(TruncateServerError::DatabaseOffline);
    };

    let (_, mut attempt) =
        get_or_create_latest_attempt(server_state, player.clone(), daily_puzzle, human_player)
            .await?;

    if attempt.won {
        return Err(TruncateServerError::PuzzleComplete);
    }

    let packed_moves = pack_moves(&moves, 2);

    if !packed_moves.starts_with(&attempt.sequence_of_moves) {
        // sacré bleu! somebody is trying to change history!
        // no sir, we will create a new attempt for these moves.
        let day_record = get_day_record(server_state, player, daily_puzzle)
            .await?
            .expect("Getting the latest attempt should have created the relevant day");
        attempt = create_new_attempt(server_state, day_record.result_id).await?;
    }

    let human_moves = moves
        .iter()
        .filter(|m| {
            let player = match m {
                Move::Place { player, .. } => player,
                Move::Swap { player, .. } => player,
            };
            *player as i32 == human_player
        })
        .count();

    // TODO: If `won` is supposedly true, we should simulate the puzzle
    // to ensure that the move sequence indeed wins

    sqlx::query!(
        "UPDATE daily_puzzle_attempts 
         SET sequence_of_moves = $1, move_count = $2, won = $3
         WHERE attempt_id = $4",
        packed_moves,
        human_moves as i32,
        won,
        attempt.attempt_id
    )
    .execute(pool)
    .await?;

    Ok(())
}
