//! Checks reward order, kill-based drop cadence, and upgrade resets between lives and levels.

use crate::game::{Game, Status};
use crate::powerups::{Reward, Upgrades};

#[test]
fn web_entry_resets_the_pickup_sequence_and_acquired_upgrades() {
    let mut upgrades = Upgrades::default();
    let drops = (1..=10)
        .filter(|_| upgrades.drop_due(3))
        .collect::<Vec<_>>();
    assert_eq!(drops, [1, 4, 7, 10]);
    upgrades.laser = true;
    upgrades.stage = 2;
    upgrades.begin_level();
    assert!(upgrades.drop_due(5));
    assert!(!upgrades.laser);
    assert_eq!(upgrades.stage, 0);
}

#[test]
fn three_webs_can_award_tokens_that_survive_death() {
    let mut upgrades = Upgrades::default();
    let mut game = Game::default();
    for tokens in 1..=3 {
        upgrades.begin_level();
        for _ in 0..6 {
            upgrades.collect(&mut game);
        }
        assert_eq!(upgrades.warp_tokens(), tokens);
    }
    upgrades.lose_life();
    assert_eq!(upgrades.warp_tokens(), 3);
    upgrades.begin_level();
    assert_eq!(upgrades.warp_tokens(), 3);
}

#[test]
fn first_kill_then_every_fifth_kill_drops_a_powerup() {
    let mut upgrades = Upgrades::default();
    let drops = (1..=25)
        .filter(|_| upgrades.drop_due(5))
        .collect::<Vec<_>>();
    assert_eq!(drops, [1, 6, 11, 16, 21]);
    assert_eq!(
        upgrades.stage, 0,
        "drops do not advance the reward sequence"
    );
}

#[test]
fn rewards_follow_collection_order_and_death_clears_upgrades() {
    let mut upgrades = Upgrades::default();
    let mut game = Game::default();
    assert_eq!(upgrades.collect(&mut game), Reward::Laser);
    assert!(upgrades.laser);
    assert_eq!(upgrades.collect(&mut game), Reward::Bonus);
    assert_eq!(game.score, 2000);
    assert_eq!(upgrades.collect(&mut game), Reward::Jump);
    assert!(upgrades.jump_enabled);
    assert_eq!(upgrades.collect(&mut game), Reward::Bonus);
    assert_eq!(game.score, 4000);
    assert_eq!(upgrades.collect(&mut game), Reward::Droid);
    assert!(upgrades.droid);
    assert_eq!(upgrades.collect(&mut game), Reward::Zap);
    assert_eq!(upgrades.warp_tokens, 1);
    upgrades.lose_life();
    assert!(!upgrades.laser && !upgrades.jump_enabled && !upgrades.droid);
    assert_eq!(upgrades.stage, 0);
    assert_eq!(upgrades.warp_tokens, 1);
}

#[test]
fn exit_pickup_moves_the_droid_to_the_first_pickup_without_advancing_the_sequence() {
    use crate::messages::Id;
    let mut upgrades = Upgrades::default();
    let mut game = Game {
        status: Status::Exiting,
        ..Game::default()
    };
    assert_eq!(upgrades.collect(&mut game), Reward::DroidNext);
    assert_eq!(upgrades.announcements, [Id::YesYesYes]);
    assert!(!upgrades.droid);
    assert_eq!((game.score, upgrades.stage), (0, 0));
    upgrades.begin_level();
    upgrades.announcements.clear();
    game.status = Status::Playing;
    assert_eq!(upgrades.collect(&mut game), Reward::Droid);
    assert_eq!(upgrades.announcements, [Id::AiDroid]);
    assert!(upgrades.droid && !upgrades.laser);
    assert_eq!(upgrades.stage, 0);
    for reward in [
        Reward::Laser,
        Reward::Bonus,
        Reward::Jump,
        Reward::Bonus,
        Reward::Zap,
    ] {
        assert_eq!(upgrades.collect(&mut game), reward);
    }
    assert_eq!((game.score, upgrades.warp_tokens()), (4000, 1));
    upgrades.begin_level();
    assert_eq!(upgrades.collect(&mut game), Reward::Laser);
    assert!(!upgrades.droid);
}

#[test]
fn pending_droid_survives_retries_and_repeated_exit_pickups_only_give_surprises() {
    use crate::messages::Id;
    let mut upgrades = Upgrades {
        stage: 5,
        droid: true,
        ..Upgrades::default()
    };
    let mut game = Game {
        status: Status::Exiting,
        ..Game::default()
    };
    assert_eq!(upgrades.collect(&mut game), Reward::DroidNext);
    assert_eq!(upgrades.collect(&mut game), Reward::Bonus);
    assert_eq!(upgrades.collect(&mut game), Reward::Bonus);
    assert_eq!(game.score, 4000);
    assert_eq!(upgrades.warp_tokens(), 0);
    assert_eq!(
        upgrades
            .announcements
            .iter()
            .filter(|id| **id == Id::YesYesYes)
            .count(),
        1
    );
    upgrades.lose_life();
    upgrades.begin_level();
    game.status = Status::Playing;
    assert_eq!(upgrades.collect(&mut game), Reward::Droid);
    upgrades.lose_life();
    assert_eq!(upgrades.collect(&mut game), Reward::Laser);
}
#[test]
fn pickups_emit_original_announcements_in_collection_order() {
    use crate::messages::Id;
    let mut upgrades = crate::powerups::Upgrades::default();
    let mut game = Game::default();
    for stage in 0..8 {
        upgrades.random = 0x2000;
        upgrades.collect(&mut game);
        let id = upgrades
            .announcements
            .drain(..)
            .find(|id| !id.reward())
            .unwrap();
        let expected = [
            Id::ParticleLaser,
            Id::Zappo,
            Id::JumpEnabled,
            Id::Outstanding,
            Id::AiDroid,
            Id::WarpTwo,
            Id::Zappo,
            Id::Dude,
        ][stage];
        assert_eq!(id, expected);
    }
}
