//! Checks reward order, kill-based drop cadence, and upgrade resets between lives and levels.

use crate::powerups::{Reward, Upgrades};

#[test]
fn configurable_drops_restart_the_counter_but_preserve_rewards_between_levels() {
    let mut upgrades = Upgrades::default();
    let drops = (1..=10)
        .filter(|_| upgrades.drop_due(3))
        .collect::<Vec<_>>();
    assert_eq!(drops, [1, 4, 7, 10]);
    upgrades.laser = true;
    upgrades.stage = 2;
    upgrades.begin_level();
    assert!(upgrades.drop_due(5));
    assert!(upgrades.laser);
    assert_eq!(upgrades.stage, 2);
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
    let mut score = 0;
    assert_eq!(upgrades.collect(&mut score), Reward::Laser);
    assert!(upgrades.laser);
    assert_eq!(upgrades.collect(&mut score), Reward::Bonus);
    assert_eq!(score, 2000);
    assert_eq!(upgrades.collect(&mut score), Reward::Jump);
    assert!(upgrades.jump_enabled);
    assert_eq!(upgrades.collect(&mut score), Reward::Bonus);
    assert_eq!(score, 4000);
    assert_eq!(upgrades.collect(&mut score), Reward::Droid);
    assert!(upgrades.droid);
    assert_eq!(upgrades.collect(&mut score), Reward::Zap);
    assert_eq!(upgrades.warp_tokens, 1);
    upgrades.lose_life();
    assert!(!upgrades.laser && !upgrades.jump_enabled && !upgrades.droid);
    assert_eq!(upgrades.stage, 0);
    assert_eq!(upgrades.warp_tokens, 1);
}
