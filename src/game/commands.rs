//! Telling the overseer what to do (O01 step 4; package P1 fills this in):
//! a spotlight card or a key gives a command, which the overseer obeys (or,
//! once O07's faith is in, may refuse). Wave 0 gives the command type and
//! where it is kept (`Overseer.commands`); the effects come with P1.

// Wave 0 interfaces for the wave-1 packages: some are not called yet.
#![allow(dead_code)]

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Raise the gate's drawbridge now.
    RaiseBridge,
    /// Send the civilians to the shelter.
    Shelter,
    /// Lower the bridge, call off the alert.
    StandDown,
    /// Take up the Muster ambition.
    Muster,
    /// Call an expedition home.
    Recall,
}

impl Command {
    pub const ALL: [Command; 5] = [Command::RaiseBridge, Command::Shelter, Command::StandDown, Command::Muster, Command::Recall];

    /// The key that gives it on a card.
    pub fn key(self) -> char {
        match self {
            Command::RaiseBridge => 'b',
            Command::Shelter => 's',
            Command::StandDown => 'd',
            Command::Muster => 'm',
            Command::Recall => 'r',
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Command::RaiseBridge => "raise the bridge",
            Command::Shelter => "to the shelter",
            Command::StandDown => "stand down",
            Command::Muster => "muster",
            Command::Recall => "recall the expedition",
        }
    }
}

impl Game {
    /// Give the overseer a command; what it says back. (Wave 0: the
    /// command is only remembered; P1 makes the overseer act on it.)
    pub fn command_overseer(&mut self, c: Command) -> String {
        let tick = self.tick;
        let Some(o) = self.overseer.as_mut() else { return "There is no overseer to command.".into() };
        o.commands.push((c, tick));
        if o.commands.len() > 20 {
            o.commands.remove(0);
        }
        format!("The overseer hears: {}.", c.label())
    }
}
