//! Wording and presentation from yak.s setmsg/drawmsg and reward objects.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub(crate) enum Id {
    CollectPowerups,
    ParticleLaser,
    JumpEnabled,
    AiDroid,
    Wicked,
    Zappo,
    Outstanding,
    Dude,
    OuttaHere,
    YesYesYes,
    WarpTwo,
    WarpOne,
    WarpEnabled,
    CaughtYou,
    ShotYou,
    FriedYou,
    Spiked,
    AvoidSpikes,
    SuperzapperRecharge,
    EatElectricDeath,
    SpeedBoost,
    GreenTrack,
    WarpFive,
    Excellent,
    Points2000,
    Points250,
    Points500,
    Points750,
    OneUp,
    GameOver,
    FinalScore,
    PracticeOver,
    YouBeatTheGame,
}

impl Id {
    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::CollectPowerups => "COLLECT POWERUPS!",
            Self::ParticleLaser => "PARTICLE LASER!",
            Self::JumpEnabled => "JUMP ENABLED!",
            Self::AiDroid => "A.I. DROID!",
            Self::Wicked => "WICKED!",
            Self::Zappo => "ZAPPO!",
            Self::Outstanding => "OUTSTANDING!",
            Self::Dude => "DUDE!",
            Self::OuttaHere => "OUTTA HERE!",
            Self::YesYesYes => "YES! YES! YES!",
            Self::WarpTwo => "2 MORE FOR WARP",
            Self::WarpOne => "1 MORE FOR WARP",
            Self::WarpEnabled => "WARP ENABLED",
            Self::CaughtYou => "CAUGHT YOU!",
            Self::ShotYou => "SHOT YOU!",
            Self::FriedYou => "FRIED YOU!",
            Self::Spiked => "SPIKED!",
            Self::AvoidSpikes => "AVOID THE SPIKES",
            Self::SuperzapperRecharge => "SUPERZAPPER\nRECHARGE",
            Self::EatElectricDeath => "EAT ELECTRIC\nDEATH!",
            Self::SpeedBoost => "SPEED BOOST",
            Self::GreenTrack => "STAY ON THE GREEN TRACK",
            Self::WarpFive => "WARP 5 LEVELS!",
            Self::Excellent => "EXCELLENT",
            Self::Points2000 => "2000",
            Self::Points250 => "250",
            Self::Points500 => "500",
            Self::Points750 => "750",
            Self::OneUp => "1UP",
            Self::GameOver => "GAME OVER",
            Self::FinalScore => "FINAL SCORE",
            Self::PracticeOver => "PRACTICE OVER",
            Self::YouBeatTheGame => "YOU BEAT THE GAME!",
        }
    }

    pub(crate) fn hold(self) -> u16 {
        match self {
            Self::AvoidSpikes | Self::SuperzapperRecharge => 250,
            _ => 100,
        }
    }
}
