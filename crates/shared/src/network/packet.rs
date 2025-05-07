use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub nickname: String,
    pub position: Vec3,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IncomingPacket {
    GetPlayers,
    PlayerConnected { name: String },
    PlayerMoved { position: Vec3 },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OutgoingPacket {
    PlayerConnected { name: String },
    PlayerMoved { name: String, position: Vec3 },
    PlayersList { players: Vec<Player> },
}
