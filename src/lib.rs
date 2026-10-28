//! The engine of the game, era-agnostic: everything about an era comes from its pack (`pack`).
//! `world` — the map, the countries and their past; `economy` — markets, trade, advisors,
//! science; `military` — battles, doctrines, the front, divisions, the home front. Small tools:
//! `rng` (deterministic randomness), `noise` (Perlin), `png` (writing pictures).

pub mod economy;
pub mod military;
pub mod noise;
pub mod pack;
pub mod png;
pub mod rng;
pub mod world;
