//! Content packs. Everything about an era — goods, kinds of people and their needs,
//! industries, deposits on the map — lives in a TOML file. The engine only knows
//! "good", "group", "sector" and "resource", so the same code runs muskets,
//! tanks or a wasteland.

use std::collections::{BTreeMap, HashMap};
use serde::Deserialize;
use crate::world::map::{Terrain, TERRAINS};

mod read;

/// Skins of the game screen a pack may choose.
pub const THEMES: [&str; 3] = ["staff", "parchment", "rust"];
/// Signs of kinds of troops on the map (as military maps draw them).
pub const SYMBOLS: [&str; 12] =
    ["infantry", "light", "armour", "wheeled-armour", "antitank", "rockets", "artillery", "mortar", "air", "antiair", "cavalry", "recon"];

/// Years of an ordinary country's science a technique of each tier takes, when the pack does not
/// say.
pub const TIER_COST: [f32; 6] = [0.0, 3.0, 6.0, 10.0, 15.0, 22.0];

pub struct Good {
    pub id: String,
    pub name: String,
    pub price: f32,
    pub military: bool,
    /// How hard it is to carry, per unit of worth lost on the way (1 = ordinary).
    pub bulk: f32,
    /// Can be carried to another country at all (services cannot).
    pub tradable: bool,
    /// On the game screen: its icon (a name in ui/icons.svg).
    pub icon: String,
}

pub struct Need {
    pub good: usize,
    /// Per million people per month.
    pub amount: f32,
    /// How much going without it hurts.
    pub weight: f32,
    /// Going without it kills.
    pub vital: bool,
}

pub struct Group {
    pub id: String,
    pub name: String,
    /// Share of the population.
    pub share: f32,
    /// Discontent this group bears before it rises.
    pub patience: f32,
    /// In order of priority: the first is bought first.
    pub needs: Vec<Need>,
    /// Groups its idle people can move into when they pay better and lack hands.
    pub becomes: Vec<usize>,
}

pub struct Sector {
    pub id: String,
    pub name: String,
    /// Which groups work here, shares summing to 1.
    pub workers: Vec<(usize, f32)>,
    /// Consumed per million workers per month; without them work stops.
    pub inputs: Vec<(usize, f32)>,
    /// (good, amount, gain): optional input that raises output by `gain` when available.
    pub boost: Vec<(usize, f32, f32)>,
    /// Produced per million workers per month.
    pub outputs: Vec<(usize, f32)>,
    /// Group that receives the profit.
    pub owner: usize,
    pub wage_share: f32,
    /// Share of the whole population employed here at the start.
    pub start: f32,
    /// Output scales with the country's fertility.
    pub land: bool,
    /// Needs this map resource; capacity limited to `per_deposit` jobs per deposit unit.
    pub deposit: Option<usize>,
    pub per_deposit: f32,
    /// Goods needed to add a million jobs of capacity.
    pub build: Vec<(usize, f32)>,
}

pub struct Resource {
    pub id: String,
    pub name: String,
    /// Chance of a deposit per province, by terrain.
    pub chance: [f32; 11],
}

/// How goods travel between countries in this era.
pub struct Trade {
    /// Group that runs trade: it keeps `margin` of what foreign goods fetch.
    pub owner: usize,
    pub margin: f32,
    /// Share of a good (of bulk 1) lost per pixel of way: at sea, and on plains
    /// (other land costs `terrain` times more), and once per loading at a port.
    pub sea: f32,
    pub land: f32,
    pub port: f32,
    /// Pixels a month.
    pub sea_speed: f32,
    pub land_speed: f32,
    /// How much harder than plains each terrain is to cross.
    pub terrain: [f32; 11],
}

/// Where a unit stands in battle.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Line {
    /// Holds the line: takes a place in the width of the front and most of the enemy's fire.
    Front,
    /// Fires from behind the line: takes no place in it and draws little fire while the line
    /// holds.
    Support,
    /// Flies over the field: no place in the line, no cover from the ground.
    Air,
}

/// A kind of unit (a battalion, a regiment, a squadron: whatever the era fields).
pub struct Unit {
    pub id: String,
    pub name: String,
    /// What it is when shot at (an index into `War::classes`).
    pub class: usize,
    pub line: Line,
    /// Thousands of men.
    pub men: f32,
    /// Goods to raise it.
    pub equip: Vec<(usize, f32)>,
    /// Goods it uses up in a month of fighting.
    pub supply: Vec<(usize, f32)>,
    /// Harm it does per day to each class of target.
    pub fire: Vec<f32>,
    /// Harm it takes to lose one unit.
    pub toughness: f32,
    /// How well it uses the cover the ground gives (1 = fully).
    pub cover: f32,
    pub speed: f32,
    /// Place it takes in the line (1 = an ordinary unit of the front).
    pub width: f32,
    /// Its fire on each terrain, against plains.
    pub terrain: [f32; 11],
    /// Worth at starting prices: equipment, six months of supply and its men.
    pub cost: f32,
    /// On the game screen: its icon (a name in ui/icons.svg) and its sign on the map (`SYMBOLS`).
    pub icon: String,
    pub symbol: String,
    /// Whether its kit has norms (`doctrine::norm`): soldiers armed more or less richly (machine
    /// guns, mortars, lorries to a battalion). Kinds whose equipment is the unit itself (tanks,
    /// guns, planes) have none: theirs is their design.
    pub norms: bool,
    /// What its equipment is called as a model of the factories («винтовка», «танк»; empty: the
    /// kind's name) — the game names its models by it.
    pub model: String,
    /// Armour and the armour its fire pierces (as in HOI4: fire on a target whose armour it does
    /// not pierce does a fraction of its harm; a unit hit by fire that does not pierce its armour
    /// loses less). 0 and 0: neither (the old battles).
    pub armor: f32,
    pub pierce: f32,
    /// Its fire when it attacks and when it defends, against its fire as given (breakthrough and
    /// defence in HOI4: tanks strike, infantry holds); 1 and 1: the same in both.
    pub attack: f32,
    pub defend: f32,
    /// How hard its equipment is to make: factories learn it this many times slower, and a new
    /// model of it costs them more of what they learned (1: an ordinary kit).
    pub complexity: f32,
}

/// A kind of warship (`[[ship]]`): what the shipyards build and how it fights at sea (the navy,
/// `military::navy`). Nothing about the era: a ship of the line, a battleship or an armoured barge —
/// whatever the pack calls it.
pub struct Ship {
    pub id: String,
    pub name: String,
    /// What it is, in words (for the shipyard's card).
    pub about: String,
    pub icon: String,
    /// Its place in a fleet (`ROLES`): "line" (heavy guns and armour: fights for the sea), "cruiser"
    /// (guns and speed: hunts merchants, guards convoys), "screen" (torpedoes and depth charges:
    /// guards the big ships and the convoys), "sub" (unseen: sinks merchants and big ships), "raider"
    /// (a light ship that lives on prizes).
    pub role: String,
    /// Goods to build one (as a unit's equipment: in the pack's figures, times `War::scale`).
    pub equip: Vec<(usize, f32)>,
    /// Goods a month at sea (in port, a third).
    pub supply: Vec<(usize, f32)>,
    /// Months on the slipway at the best.
    pub months: u32,
    /// Its crew, thousands of men.
    pub crew: f32,
    /// Its guns against ships, its armour (guns lighter than it do a fraction of their harm), the
    /// harm it takes to sink, torpedoes (they pierce any armour), its weapons against submarines,
    /// how hard it is to find (0 — always seen … 1 — never), its speed.
    pub gun: f32,
    pub armor: f32,
    pub hull: f32,
    pub torpedo: f32,
    pub asw: f32,
    pub stealth: f32,
    pub speed: f32,
    /// What it does to the enemy's merchants in a month of raiding, and what it does to keep raiders
    /// off its own convoys.
    pub raid: f32,
    pub escort: f32,
    /// Worth at starting prices: its building and six months at sea.
    pub cost: f32,
}

/// The places of a ship in a fleet, and their names in words.
pub const ROLES: [(&str, &str); 5] = [("line", "линейные силы"), ("cruiser", "крейсера"), ("screen", "лёгкие силы"), ("sub", "подводные"), ("raider", "рейдеры")];

/// A company of support (`[[support]]`): one to a division, it serves the whole of it — a big
/// division gets as much from it as a small one, for the same price. Used by the war of
/// divisions (`corps`) only.
pub struct Support {
    pub id: String,
    pub name: String,
    /// What it does, in words (for the designer).
    pub about: String,
    pub icon: String,
    /// Men, in units of the pack (a battalion is a tenth of one).
    pub men: f32,
    pub equip: Vec<(usize, f32)>,
    pub supply: Vec<(usize, f32)>,
    /// The division digs in this many times as fast.
    pub dig: f32,
    /// It crosses rough ground (forest, hills, marsh, jungle, mountains) in this share of the days.
    pub rough: f32,
    /// Provinces deeper it sees the enemy through the fog (0: only next to it).
    pub see: f32,
    /// It gets its organisation back this many times as fast.
    pub org: f32,
    /// Share of its dead it saves (the wounded come back to the ranks).
    pub saved: f32,
    /// It eats this share of the supply (what a crowded province can feed).
    pub eat: f32,
    /// Its fire, this many times.
    pub fire: f32,
    /// Share of the equipment it would leave behind in a retreat that it saves; trophies taken.
    pub salvage: f32,
    /// Worth at starting prices: equipment, six months of supply, its men.
    pub cost: f32,
}

/// How battles go in this era.
pub struct War {
    /// Classes of targets, as the pack names them.
    pub classes: Vec<String>,
    /// Places in the line on plains (an ordinary unit of the front takes one); elsewhere
    /// `frontage` times as many.
    pub width: f32,
    pub frontage: [f32; 11],
    /// Share of harm the ground keeps off a defender who uses it fully.
    pub cover: [f32; 11],
    /// The longest a battle lasts, in days.
    pub days: u32,
    /// Worth of a thousand men, against goods at starting prices.
    pub man_cost: f32,
    /// How much of a good of the market one of a unit's figures of equipment and supply stands
    /// for (the market counts goods by what a million people use in a month; a battalion does
    /// not eat a million people's bread).
    pub scale: f32,
    /// How many people one of a unit's figure of men stands for, in thousands (the front's unit
    /// is a formation; its men matter only to what it costs and whom it takes from work).
    pub crew: f32,
}

pub struct Pack {
    pub name: String,
    pub about: String,
    pub tax: f32,
    /// Share of the treasury spent on military goods.
    pub military: f32,
    pub goods: Vec<Good>,
    pub groups: Vec<Group>,
    pub sectors: Vec<Sector>,
    pub resources: Vec<Resource>,
    /// None: countries do not trade.
    pub trade: Option<Trade>,
    /// None: this pack fields no armies.
    pub war: Option<War>,
    pub units: Vec<Unit>,
    /// Companies of support a division may have (the war of divisions only).
    pub support: Vec<Support>,
    /// Kinds of warships (empty: this pack fields no navy).
    pub ships: Vec<Ship>,
    /// None: this pack has no science.
    pub science: Option<Science>,
    /// The skin of the game screen (`THEMES`).
    pub theme: String,
    /// Millions of people a world of this era holds, the least and the most (each world draws its
    /// own between them).
    pub people: [f32; 2],
    /// The techniques of the era (the pack's, or ones made of its sectors and units when it names
    /// none), and the names of its stages of development.
    pub techs: Vec<Tech>,
    pub stages: Vec<String>,
}

/// The sphere a technique belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sphere {
    Economy,
    Military,
    Science,
    State,
}

impl Sphere {
    pub fn word(&self) -> &'static str {
        match self {
            Sphere::Economy => "хозяйство",
            Sphere::Military => "военное дело",
            Sphere::Science => "наука",
            Sphere::State => "государство",
        }
    }
}

/// A technique of the era.
#[derive(Clone, Debug)]
pub struct Tech {
    pub id: String,
    pub name: String,
    pub about: String,
    pub icon: String,
    pub tier: usize,
    pub sphere: Sphere,
    pub needs: Vec<usize>,
    /// Years of an ordinary country's science it takes.
    pub cost: f32,
    /// Productivity of sectors and fire and toughness of kinds of units it adds (logs).
    pub sectors: Vec<(usize, f32)>,
    pub units: Vec<(usize, f32)>,
    pub unlocks: Vec<usize>,
    /// Shares added: science's power, factories' learning, stability a year, the draft, supply.
    pub science: f32,
    pub learn: f32,
    pub stability: f32,
    pub draft: f32,
    pub supply: f32,
}

/// Who can do science, and what share of the state's spending goes to it by custom.
pub struct Science {
    /// Groups whose people can work in laboratories.
    pub learned: Vec<usize>,
    pub share: f32,
}

impl Pack {
    /// Whether good `i` is a ration: something people cannot live without (bread, water) — troops
    /// eat it by their men, whatever their kit; the rest of their supply (ammunition, fuel) they
    /// burn by their kit.
    pub fn ration(&self, i: usize) -> bool {
        self.groups.iter().any(|g| g.needs.iter().any(|n| n.good == i && n.vital))
    }

    pub fn load(path: &str) -> Result<Pack, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        Pack::parse(&text).map_err(|e| format!("{path}: {e}"))
    }

}

