//! Parse the original PDP `advent.dat` (the 350-point Crowther & Woods data)
//! into immutable game tables.
//!
//! This is a direct Rust port of `brandon-rhodes/python-adventure`'s `data.py`
//! and `model.py` (Apache-2.0; the game text itself is public domain). The data
//! file is the canonical 12-section format; see the section doc-comments below
//! for the layout. The whole thing is parsed once, lazily, into a shared
//! [`GameData`] behind a [`OnceLock`]; per-playthrough mutable state lives in
//! `Game` (see `state.rs`).

use std::collections::HashMap;
use std::sync::OnceLock;

/// The verbatim 1977/1995 350-point data file, embedded in the binary.
const ADVENT_DAT: &str = include_str!("advent.dat");

/// The highest room number in the data file.
#[allow(dead_code)]
pub const MAX_ROOM: usize = 140;

// Object numbers (stable in the data file). Names mirror python-adventure's
// `self.<name>` attributes, which come from each object's first vocabulary word.
pub const KEYS: usize = 1;
pub const LAMP: usize = 2;
pub const GRATE: usize = 3;
pub const CAGE: usize = 4;
pub const ROD: usize = 5;
pub const ROD2: usize = 6;
pub const STEPS: usize = 7;
pub const BIRD: usize = 8;
pub const DOOR: usize = 9;
pub const PILLOW: usize = 10;
pub const SNAKE: usize = 11;
pub const FISSURE: usize = 12;
pub const TABLET: usize = 13;
pub const CLAM: usize = 14;
pub const OYSTER: usize = 15;
pub const MAGAZINE: usize = 16;
pub const DWARF: usize = 17;
pub const KNIFE: usize = 18;
pub const FOOD: usize = 19;
pub const BOTTLE: usize = 20;
pub const WATER: usize = 21;
pub const OIL: usize = 22;
pub const MIRROR: usize = 23;
pub const PLANT: usize = 24;
pub const PLANT2: usize = 25;
pub const STALACTITE: usize = 26;
pub const SHADOW: usize = 27;
pub const AXE: usize = 28;
pub const DRAWINGS: usize = 29;
pub const PIRATE_OBJ: usize = 30;
pub const DRAGON: usize = 31;
pub const CHASM: usize = 32;
pub const TROLL: usize = 33;
pub const TROLL2: usize = 34;
pub const BEAR: usize = 35;
pub const MESSAGE: usize = 36;
pub const VOLCANO: usize = 37;
pub const MACHINE: usize = 38;
pub const BATTERIES: usize = 39;
pub const CARPET: usize = 40;
pub const GOLD: usize = 50;
pub const DIAMONDS: usize = 51;
pub const SILVER: usize = 52;
pub const JEWELRY: usize = 53;
pub const COINS: usize = 54;
pub const CHEST: usize = 55;
pub const EGGS: usize = 56;
pub const TRIDENT: usize = 57;
pub const VASE: usize = 58;
pub const EMERALD: usize = 59;
pub const PLATINUM: usize = 60;
pub const PEARL: usize = 61;
pub const RUG: usize = 62;
pub const SPICES: usize = 63;
pub const CHAIN: usize = 64;
#[allow(dead_code)]
pub const MAX_OBJ: usize = 64;

/// Every named object number, listed so the full table is documented (and
/// referenced) even where a scenery object carries no special game logic.
#[allow(dead_code)]
const OBJECT_TABLE: [usize; 55] = [
    KEYS, LAMP, GRATE, CAGE, ROD, ROD2, STEPS, BIRD, DOOR, PILLOW, SNAKE, FISSURE, TABLET, CLAM,
    OYSTER, MAGAZINE, DWARF, KNIFE, FOOD, BOTTLE, WATER, OIL, MIRROR, PLANT, PLANT2, STALACTITE,
    SHADOW, AXE, DRAWINGS, PIRATE_OBJ, DRAGON, CHASM, TROLL, TROLL2, BEAR, MESSAGE, VOLCANO, MACHINE,
    BATTERIES, CARPET, GOLD, DIAMONDS, SILVER, JEWELRY, COINS, CHEST, EGGS, TRIDENT, VASE, EMERALD,
    PLATINUM, PEARL, RUG, SPICES, CHAIN,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Travel,
    Noun,
    Verb,
    Snappy,
}

/// The destination half of a travel-table entry.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Room(u16),
    Message(i32),
    Special(i32),
}

/// The condition half of a travel-table entry (decoded from the `M` of `Y`).
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Always,
    Percent(i32),
    NotDwarf,
    Carrying(usize),
    CarryingOrWith(usize),
    /// `PROP(obj)` must not equal this value.
    PropNot(usize, i32),
}

#[derive(Debug, Clone)]
pub struct Move {
    pub is_forced: bool,
    pub verbs: Vec<i32>,
    pub condition: Condition,
    pub action: Action,
}

#[derive(Debug, Clone, Default)]
pub struct RoomData {
    pub long: String,
    pub short: String,
    pub travel: Vec<Move>,
    pub is_light: bool,
    pub liquid: Option<usize>,
    pub is_forbidden_to_pirate: bool,
}

impl RoomData {
    pub fn is_forced(&self) -> bool {
        self.travel.first().map(|m| m.is_forced).unwrap_or(false)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ObjData {
    pub names: Vec<String>,
    pub is_treasure: bool,
    pub inventory_message: String,
    /// prop value -> description shown when the object lies in a room.
    pub messages: HashMap<i32, String>,
    pub start_rooms: Vec<u16>,
    pub start_fixed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct HintData {
    pub n: i32,
    pub turns_needed: i32,
    pub penalty: i32,
    pub question: i32,
    pub message: i32,
    pub rooms: Vec<u16>,
}

#[derive(Debug)]
pub struct GameData {
    pub rooms: Vec<RoomData>,
    pub objects: Vec<ObjData>,
    pub messages: HashMap<i32, String>,
    pub class_messages: Vec<(i32, String)>,
    pub hints: Vec<HintData>,
    /// every word text (and its 5-char truncation) -> vocabulary number `n`.
    pub text_to_n: HashMap<String, i32>,
    /// vocabulary number -> ordered synonym texts (first is canonical).
    pub groups: HashMap<i32, Vec<String>>,
    /// verb number (2000+x) -> default message number.
    pub default_msg: HashMap<i32, i32>,
    /// sorted, de-duplicated full word spellings, for input autocomplete.
    pub display_words: Vec<String>,
}

/// Vocabulary words (full spellings) that start with `prefix`, for the UI's
/// autocomplete chip strip. Empty `prefix` yields a handy default set.
pub fn autocomplete(prefix: &str, limit: usize) -> Vec<String> {
    let d = data();
    let p = prefix.trim().to_lowercase();
    if p.is_empty() {
        return DEFAULT_SUGGESTIONS.iter().map(|s| s.to_string()).collect();
    }
    d.display_words
        .iter()
        .filter(|w| w.starts_with(&p))
        .take(limit)
        .cloned()
        .collect()
}

/// Shown when the command box is empty — the moves a new player reaches for.
const DEFAULT_SUGGESTIONS: [&str; 10] = [
    "look", "north", "south", "east", "west", "up", "down", "take", "drop", "inventory",
];

impl GameData {
    pub fn kind(&self, n: i32) -> Kind {
        match n / 1000 {
            0 => Kind::Travel,
            1 => Kind::Noun,
            2 => Kind::Verb,
            _ => Kind::Snappy,
        }
    }

    /// The canonical (first) synonym text for a vocabulary number.
    pub fn canonical(&self, n: i32) -> &str {
        self.groups
            .get(&n)
            .and_then(|g| g.first())
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// python's `word == text`: true when `text` is one of `n`'s synonyms.
    pub fn word_is(&self, n: i32, text: &str) -> bool {
        self.text_to_n.get(text) == Some(&n)
    }

    pub fn message(&self, n: i32) -> String {
        self.messages.get(&n).cloned().unwrap_or_default()
    }
}

/// The shared, parsed data tables. Parsed once on first use.
pub fn data() -> &'static GameData {
    static DATA: OnceLock<GameData> = OnceLock::new();
    DATA.get_or_init(parse)
}

// The data file knows only the first five characters of each word, so we
// restore the full spelling of any word the game logic refers to by name.
const LONG_WORDS: &str = "upstream downstream forest \
forward continue onward return retreat valley staircase outside building stream \
cobble inward inside surface nowhere passage tunnel canyon awkward \
upward ascend downward descend outdoors barren across debris broken \
examine describe slabroom depression entrance secret bedquilt plover \
oriental cavern reservoir office headlamp lantern pillow velvet fissure tablet \
oyster magazine spelunker dwarves knives rations bottle mirror beanstalk \
stalactite shadow figure drawings pirate dragon message volcano geyser \
machine vending batteries carpet nuggets diamonds silver jewelry treasure \
trident shards pottery emerald platinum pyramid pearl persian spices capture \
release discard mumble unlock nothing extinguish placate travel proceed \
continue explore follow attack strike devour inventory detonate ignite \
blowup peruse shatter disturb suspend sesame opensesame abracadabra \
shazam excavate information";

fn long_words() -> HashMap<String, String> {
    LONG_WORDS
        .split_whitespace()
        .map(|w| (w[..w.len().min(5)].to_string(), w.to_string()))
        .collect()
}

/// `expand_tabs`: rejoin text segments at 8-column tab stops.
fn expand_tabs(segments: &[&str]) -> String {
    let mut it = segments.iter();
    let mut line = match it.next() {
        Some(s) => s.to_string(),
        None => return String::new(),
    };
    for segment in it {
        let spaces = 8 - line.chars().count() % 8;
        line.push_str(&" ".repeat(spaces));
        line.push_str(segment);
    }
    line
}

fn ensure_room(rooms: &mut Vec<RoomData>, n: usize) -> &mut RoomData {
    if n >= rooms.len() {
        rooms.resize_with(n + 1, RoomData::default);
    }
    &mut rooms[n]
}

fn ensure_obj(objects: &mut Vec<ObjData>, n: usize) -> &mut ObjData {
    if n >= objects.len() {
        objects.resize_with(n + 1, ObjData::default);
    }
    &mut objects[n]
}

fn parse() -> GameData {
    let lw = long_words();
    let mut rooms: Vec<RoomData> = Vec::new();
    let mut objects: Vec<ObjData> = Vec::new();
    let mut messages: HashMap<i32, String> = HashMap::new();
    let mut class_messages: Vec<(i32, String)> = Vec::new();
    let mut hints_map: HashMap<i32, HintData> = HashMap::new();
    let mut text_to_n: HashMap<String, i32> = HashMap::new();
    let mut groups: HashMap<i32, Vec<String>> = HashMap::new();
    let mut default_msg: HashMap<i32, i32> = HashMap::new();

    // Section-3 state: "same x and same first verb implies use whole list".
    let mut last_travel: (i64, Vec<i64>) = (0, vec![0]);
    // Section-5 state: the object whose prop messages we are currently reading.
    let mut cur_obj: usize = 0;

    let mut lines = ADVENT_DAT.lines();
    while let Some(raw_section) = lines.next() {
        let section_line = raw_section.trim();
        if section_line.is_empty() {
            continue;
        }
        let section: i32 = section_line.parse().unwrap_or(0);
        if section == 0 {
            break;
        }
        for raw in lines.by_ref() {
            let line = raw.trim();
            let fields: Vec<&str> = line.split('\t').collect();
            if fields[0] == "-1" {
                break;
            }
            let n: i64 = fields[0].parse().unwrap_or(0);
            let rest = &fields[1..];
            match section {
                1 => {
                    let room = ensure_room(&mut rooms, n as usize);
                    if !rest.first().map(|s| s.starts_with(">$<")).unwrap_or(true) {
                        room.long.push_str(&expand_tabs(rest));
                        room.long.push('\n');
                    }
                }
                2 => {
                    let room = ensure_room(&mut rooms, n as usize);
                    room.short.push_str(&expand_tabs(rest));
                    room.short.push('\n');
                }
                3 => section3(
                    &mut rooms,
                    &mut groups,
                    n,
                    rest,
                    &mut last_travel,
                ),
                4 => section4(
                    &lw,
                    &mut objects,
                    &mut text_to_n,
                    &mut groups,
                    n,
                    rest,
                ),
                5 => {
                    if (1..=99).contains(&n) {
                        cur_obj = n as usize;
                        ensure_obj(&mut objects, cur_obj).inventory_message = expand_tabs(rest);
                    } else {
                        let prop = (n / 100) as i32;
                        let more = if rest.first().map(|s| s.starts_with(">$<")).unwrap_or(true) {
                            String::new()
                        } else {
                            let mut s = expand_tabs(rest);
                            s.push('\n');
                            s
                        };
                        let obj = ensure_obj(&mut objects, cur_obj);
                        obj.messages.entry(prop).or_default().push_str(&more);
                    }
                }
                6 => {
                    let entry = messages.entry(n as i32).or_default();
                    entry.push_str(&expand_tabs(rest));
                    entry.push('\n');
                }
                7 => {
                    let obj_n = n as usize;
                    let room_n = rest.first().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
                    let fixed = rest.get(1).and_then(|s| s.parse::<i64>().ok());
                    let obj = ensure_obj(&mut objects, obj_n);
                    if room_n != 0 {
                        obj.start_rooms = vec![room_n as u16];
                    }
                    if let Some(f) = fixed {
                        if f == -1 {
                            obj.start_fixed = true;
                        } else {
                            // Two locations (e.g. the grate). python doesn't set
                            // is_fixed here; immovability falls out of len>1.
                            obj.start_rooms.push(f as u16);
                        }
                    }
                }
                8 => {
                    let word_n = (n + 2000) as i32;
                    let msg_n = rest.first().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                    if msg_n != 0 {
                        default_msg.insert(word_n, msg_n);
                    }
                }
                9 => {
                    let bit = n as i32;
                    for f in rest {
                        // Room numbers here are non-negative; parse straight to
                        // usize and skip anything that isn't (a stray sign or
                        // overflowing token would otherwise panic the parse).
                        let Ok(rn) = f.parse::<usize>() else {
                            continue;
                        };
                        match bit {
                            0 => ensure_room(&mut rooms, rn).is_light = true,
                            1 => ensure_room(&mut rooms, rn).liquid = Some(OIL),
                            2 => ensure_room(&mut rooms, rn).liquid = Some(WATER),
                            3 => ensure_room(&mut rooms, rn).is_forbidden_to_pirate = true,
                            _ => hints_map
                                .entry(bit)
                                .or_insert_with(|| HintData {
                                    n: bit,
                                    ..Default::default()
                                })
                                .rooms
                                .push(rn as u16),
                        }
                    }
                }
                10 => {
                    class_messages.push((n as i32, expand_tabs(rest)));
                }
                11 => {
                    let nums: Vec<i32> =
                        rest.iter().filter_map(|s| s.parse::<i32>().ok()).collect();
                    let h = hints_map.entry(n as i32).or_insert_with(|| HintData {
                        n: n as i32,
                        ..Default::default()
                    });
                    h.turns_needed = nums.first().copied().unwrap_or(0);
                    h.penalty = nums.get(1).copied().unwrap_or(0);
                    h.question = nums.get(2).copied().unwrap_or(0);
                    h.message = nums.get(3).copied().unwrap_or(0);
                }
                12 => { /* magic/maintenance messages: unused in gameplay */ }
                _ => {}
            }
        }
    }

    // Add the five-letter truncations the game accepts (Game.start does this).
    let truncations: Vec<(String, i32)> = text_to_n
        .iter()
        .filter(|(k, _)| k.chars().count() > 5)
        .map(|(k, v)| (k.chars().take(5).collect::<String>(), *v))
        .collect();
    for (k, v) in truncations {
        text_to_n.entry(k).or_insert(v);
    }

    let mut hints: Vec<HintData> = hints_map.into_values().collect();
    hints.sort_by_key(|h| h.n);

    // Full word spellings for autocomplete: every synonym, alphabetic only.
    let mut display_words: Vec<String> = groups
        .values()
        .flatten()
        .filter(|w| w.chars().all(|c| c.is_ascii_lowercase()))
        .cloned()
        .collect();
    display_words.sort();
    display_words.dedup();

    GameData {
        rooms,
        objects,
        messages,
        class_messages,
        hints,
        text_to_n,
        groups,
        default_msg,
        display_words,
    }
}

fn section3(
    rooms: &mut Vec<RoomData>,
    groups: &mut HashMap<i32, Vec<String>>,
    x: i64,
    rest: &[&str],
    last_travel: &mut (i64, Vec<i64>),
) {
    let y: i64 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut verbs: Vec<i64> = rest[1..].iter().filter_map(|s| s.parse::<i64>().ok()).collect();

    if last_travel.0 == x && last_travel.1.first() == verbs.first() {
        verbs = last_travel.1.clone();
    } else {
        *last_travel = (x, verbs.clone());
    }

    let m = y / 1000;
    let n = y % 1000;
    let mh = m / 100;
    let mm = (m % 100) as usize;

    let condition = if m == 0 {
        Condition::Always
    } else if (1..100).contains(&m) {
        Condition::Percent(m as i32)
    } else if m == 100 {
        Condition::NotDwarf
    } else if (101..=200).contains(&m) {
        Condition::Carrying(mm)
    } else if (201..=300).contains(&m) {
        Condition::CarryingOrWith(mm)
    } else {
        Condition::PropNot(mm, (mh - 3) as i32)
    };

    let action = if n <= 300 {
        Action::Room(n as u16)
    } else if n <= 500 {
        Action::Special(n as i32)
    } else {
        Action::Message((n - 500) as i32)
    };

    let (is_forced, kept_verbs) = if verbs.len() == 1 && verbs[0] == 1 {
        (true, Vec::new())
    } else {
        // Touch `groups` so synonym tables stay populated even for travel verbs
        // that never appear in section 4 first (defensive; no-op normally).
        let kept: Vec<i32> = verbs.iter().copied().filter(|&v| v < 100).map(|v| v as i32).collect();
        for &v in &kept {
            groups.entry(v).or_default();
        }
        (false, kept)
    };

    ensure_room(rooms, x as usize).travel.push(Move {
        is_forced,
        verbs: kept_verbs,
        condition,
        action,
    });
}

fn section4(
    lw: &HashMap<String, String>,
    objects: &mut Vec<ObjData>,
    text_to_n: &mut HashMap<String, i32>,
    groups: &mut HashMap<i32, Vec<String>>,
    n: i64,
    rest: &[&str],
) {
    let raw = rest.first().copied().unwrap_or("").to_lowercase();
    let text = lw.get(&raw).cloned().unwrap_or(raw);
    let nn = n as i32;

    let g = groups.entry(nn).or_default();
    if !g.contains(&text) {
        g.push(text.clone());
    }

    if nn / 1000 == 1 {
        let obj_n = (nn % 1000) as usize;
        let obj = ensure_obj(objects, obj_n);
        obj.names.push(text.clone());
        obj.is_treasure = obj_n >= 50;
    }

    // python: vocabulary[text] = word only the first time a text is seen.
    text_to_n.entry(text).or_insert(nn);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_expected_table_sizes() {
        let d = data();
        // 140 rooms (plus index 0).
        assert_eq!(d.rooms.len(), MAX_ROOM + 1);
        assert!(d.objects.len() > MAX_OBJ);
        // Spot-check the opening room text.
        assert!(d.rooms[1].long.starts_with("YOU ARE STANDING AT THE END OF A ROAD"));
        assert!(d.rooms[1].short.starts_with("YOU'RE AT END OF ROAD"));
    }

    #[test]
    fn vocabulary_and_kinds() {
        let d = data();
        assert_eq!(d.kind(d.text_to_n["take"]), Kind::Verb);
        assert_eq!(d.kind(d.text_to_n["north"]), Kind::Travel);
        assert_eq!(d.kind(d.text_to_n["lamp"]), Kind::Noun);
        assert_eq!(d.kind(d.text_to_n["help"]), Kind::Snappy);
        // Canonical verb names match python's method suffixes.
        assert_eq!(d.canonical(d.text_to_n["take"]), "carry");
        assert_eq!(d.canonical(d.text_to_n["get"]), "carry");
        assert_eq!(d.canonical(d.text_to_n["inventory"]), "inventory");
        // Five-letter truncation works ("inven" -> inventory verb).
        assert_eq!(d.text_to_n.get("inven"), d.text_to_n.get("inventory"));
        // word_is matches synonyms.
        let take = d.text_to_n["take"];
        assert!(d.word_is(take, "get"));
        assert!(!d.word_is(take, "drop"));
    }

    #[test]
    fn objects_and_treasures() {
        let d = data();
        assert!(d.objects[LAMP].names.contains(&"lamp".to_string()));
        assert!(!d.objects[LAMP].is_treasure);
        assert!(d.objects[GOLD].is_treasure);
        assert!(d.objects[CHAIN].is_treasure);
        // Grate starts in two rooms; immovability falls out of len>1, so the
        // explicit fixed flag stays false (matching python-adventure).
        assert_eq!(d.objects[GRATE].start_rooms.len(), 2);
        assert!(!d.objects[GRATE].start_fixed);
        // The lamp is immovable-free and the snake is fixed (-1 in section 7).
        assert!(d.objects[SNAKE].start_fixed);
        // Lamp inventory message exists.
        assert!(!d.objects[LAMP].inventory_message.is_empty());
    }

    #[test]
    fn travel_grate_special() {
        let d = data();
        // Room 1 (end of road) has travel entries.
        assert!(!d.rooms[1].travel.is_empty());
        // The light rooms include the aboveground 1..8.
        for r in 1..=8 {
            assert!(d.rooms[r].is_light, "room {r} should be light");
        }
        // Room 8 (outside grate) is a hint area; hints parsed.
        assert!(d.hints.iter().any(|h| h.n == 4));
    }
}
