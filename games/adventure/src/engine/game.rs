//! The Colossal Cave game state machine.
//!
//! This is a faithful Rust port of `brandon-rhodes/python-adventure`'s
//! `game.py` (Apache-2.0; game text public domain), which itself mirrors Don
//! Woods' `advent.for`. Python object identity becomes object/room *indices*;
//! the `yesno_callback` closure becomes a serializable [`Pending`] tag we
//! dispatch on. Random-number calls happen in exactly the same order as the
//! reference so the seeded walkthroughs replay bit-for-bit (see `tests.rs`).
//!
//! `#NNNN` comments cite FORTRAN line numbers, kept from python-adventure.

use serde::{Deserialize, Serialize};

use super::data::*;
use super::rng::PyRandom;
use super::state::{Line, Mode, Pending};

const ROOM_BUILDING: u16 = 3;
const CHEST_ROOM: u16 = 114;

/// Cap on the retained scrollback. The transcript is serialized into every save
/// and re-rendered each turn, so leaving it unbounded would grow both the saved
/// payload and the per-render cost without limit over a long session; we keep
/// only the most recent lines (far more than fits on any screen).
const MAX_TRANSCRIPT_LINES: usize = 500;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Dwarf {
    pub room: u16,
    pub old_room: u16,
    pub has_seen: bool,
    pub is_pirate: bool,
}

impl Dwarf {
    fn new(room: u16, is_pirate: bool) -> Self {
        Dwarf {
            room,
            old_room: room,
            has_seen: false,
            is_pirate,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub mode: Mode,
    pub seed: u64,
    rng: PyRandom,
    pub transcript: Vec<Line>,

    // Output buffer for the command currently being processed.
    #[serde(skip)]
    buf: String,

    pub pending: Option<Pending>,
    pending_casual: bool,

    pub loc: u16,
    oldloc: u16,
    oldloc2: u16,

    obj_prop: Vec<i32>,
    obj_rooms: Vec<Vec<u16>>,
    obj_toting: Vec<bool>,
    obj_fixed: Vec<i32>,
    bottle_contents: Option<usize>,

    times_described: Vec<i32>,

    dwarves: Vec<Dwarf>,
    pirate: Dwarf,

    hint_counter: Vec<i32>,
    hint_used: Vec<bool>,

    look_complaints: i32,
    full_description_period: i32,
    full_wests: i32,
    dwarf_stage: i32,
    dwarves_killed: i32,
    knife_location: Option<u16>,
    foobar: i32,
    gave_up: bool,
    treasures_not_found: i32,
    impossible_treasures: i32,
    lamp_turns: i32,
    warned_about_dim_lamp: bool,
    bonus: i32,
    is_dead: bool,
    deaths: i32,
    max_deaths: i32,
    turns: i32,

    clock1: i32,
    clock2: i32,
    is_closing: bool,
    panic: bool,
    is_closed: bool,
    is_done: bool,
    could_fall_in_pit: bool,

    /// UI guard: set once the finished game has been written to the score table.
    #[serde(default)]
    pub score_recorded: bool,
}

impl Game {
    // ----- construction ---------------------------------------------------

    /// A fresh, un-started game sitting behind the splash screen.
    pub fn new(seed: u64) -> Self {
        let d = data();
        let nobj = d.objects.len();
        let mut g = Game {
            mode: Mode::Splash,
            seed,
            rng: PyRandom::from_seed(seed),
            transcript: Vec::new(),
            buf: String::new(),
            pending: None,
            pending_casual: false,
            loc: 1,
            oldloc: 1,
            oldloc2: 1,
            obj_prop: vec![0; nobj],
            obj_rooms: vec![Vec::new(); nobj],
            obj_toting: vec![false; nobj],
            obj_fixed: vec![0; nobj],
            bottle_contents: None,
            times_described: vec![0; d.rooms.len()],
            dwarves: Vec::new(),
            pirate: Dwarf::default(),
            hint_counter: vec![0; d.hints.len()],
            hint_used: vec![false; d.hints.len()],
            look_complaints: 3,
            full_description_period: 5,
            full_wests: 0,
            dwarf_stage: 0,
            dwarves_killed: 0,
            knife_location: None,
            foobar: -1,
            gave_up: false,
            treasures_not_found: 0,
            impossible_treasures: 0,
            lamp_turns: 330,
            warned_about_dim_lamp: false,
            bonus: 0,
            is_dead: false,
            deaths: 0,
            max_deaths: 3,
            turns: 0,
            clock1: 30,
            clock2: 50,
            is_closing: false,
            panic: false,
            is_closed: false,
            is_done: false,
            could_fall_in_pit: false,
            score_recorded: false,
        };
        for o in 1..nobj {
            g.obj_rooms[o] = d.objects[o].start_rooms.clone();
            g.obj_fixed[o] = if d.objects[o].start_fixed { 1 } else { 0 };
        }
        g
    }

    /// A new game already past the title screen, with the welcome shown.
    pub fn started(seed: u64) -> Self {
        let mut g = Game::new(seed);
        g.mode = Mode::Playing;
        g.buf.clear();
        g.start();
        let out = std::mem::take(&mut g.buf);
        if !out.is_empty() {
            g.push_line(Line::Out(out));
        }
        g
    }

    fn start(&mut self) {
        // #1018: the 5-letter truncations are baked into the parsed vocabulary.
        self.bottle_contents = Some(WATER);
        let q = data().message(65);
        self.w(q);
        self.pending = Some(Pending::Instructions);
        self.pending_casual = false;
    }

    fn start2(&mut self, yes: bool) {
        if yes {
            self.wm(1);
            if let Some(i) = self.hint_idx(3) {
                self.hint_used[i] = true;
            }
            self.lamp_turns = 1000;
        }
        self.loc = 1;
        self.oldloc = 1;
        self.oldloc2 = 1;
        self.dwarves = [19u16, 27, 33, 44, 64]
            .iter()
            .map(|&r| Dwarf::new(r, false))
            .collect();
        self.pirate = Dwarf::new(CHEST_ROOM, true);
        let treasures = self.treasures();
        self.treasures_not_found = treasures.len() as i32;
        for t in treasures {
            self.obj_prop[t] = -1;
        }
        self.describe_location();
    }

    // ----- tiny helpers ---------------------------------------------------

    /// Append one line to the scrollback, trimming the oldest lines once it grows
    /// past [`MAX_TRANSCRIPT_LINES`]. The one place the transcript grows.
    fn push_line(&mut self, line: Line) {
        self.transcript.push(line);
        let overflow = self.transcript.len().saturating_sub(MAX_TRANSCRIPT_LINES);
        if overflow > 0 {
            self.transcript.drain(0..overflow);
        }
    }

    fn w(&mut self, more: String) {
        self.w_str(&more);
    }

    /// `w` for borrowed text — avoids cloning `&'static` data (e.g. room
    /// descriptions) just to hand `w` an owned `String`.
    fn w_str(&mut self, more: &str) {
        if !more.is_empty() {
            self.buf.push_str(&more.to_uppercase());
            self.buf.push('\n');
        }
    }

    fn wm(&mut self, n: i32) {
        let s = data().message(n);
        self.w(s);
    }

    fn is(&self, w: &W, text: &str) -> bool {
        data().word_is(w.n, text)
    }

    fn hint_idx(&self, n: i32) -> Option<usize> {
        data().hints.iter().position(|h| h.n == n)
    }

    fn referent(&self, n: i32) -> usize {
        (n % 1000) as usize
    }

    fn is_at(&self, obj: usize, room: u16) -> bool {
        self.obj_rooms[obj].contains(&room)
    }

    fn is_here(&self, obj: usize) -> bool {
        self.obj_toting[obj] || self.obj_rooms[obj].contains(&self.loc)
    }

    fn carry(&mut self, obj: usize) {
        self.obj_rooms[obj].clear();
        self.obj_toting[obj] = true;
    }

    fn drop_at(&mut self, obj: usize, room: u16) {
        self.obj_rooms[obj] = vec![room];
        self.obj_toting[obj] = false;
    }

    fn hide(&mut self, obj: usize) {
        self.obj_rooms[obj].clear();
        self.obj_toting[obj] = false;
    }

    fn objects_at(&self, room: u16) -> Vec<usize> {
        (1..self.obj_rooms.len())
            .filter(|&o| self.is_at(o, room))
            .collect()
    }

    fn objects_here(&self) -> Vec<usize> {
        self.objects_at(self.loc)
    }

    fn inventory(&self) -> Vec<usize> {
        (1..self.obj_toting.len())
            .filter(|&o| self.obj_toting[o])
            .collect()
    }

    fn treasures(&self) -> Vec<usize> {
        let d = data();
        (1..d.objects.len())
            .filter(|&o| d.objects[o].is_treasure)
            .collect()
    }

    fn is_dark(&self) -> bool {
        if self.is_here(LAMP) && self.obj_prop[LAMP] != 0 {
            return false;
        }
        !data().rooms[self.loc as usize].is_light
    }

    fn loc_is_forced(&self) -> bool {
        data().rooms[self.loc as usize].is_forced()
    }

    fn room_is_forced(&self, n: u16) -> bool {
        data().rooms[n as usize].is_forced()
    }

    fn loc_liquid(&self) -> Option<usize> {
        data().rooms[self.loc as usize].liquid
    }

    fn dwarf_at_loc(&self) -> bool {
        self.dwarves.iter().any(|d| d.room == self.loc)
    }

    fn default_msg(&self, verb: &W) -> String {
        let d = data();
        match d.default_msg.get(&verb.n) {
            Some(&m) => d.message(m),
            None => String::new(),
        }
    }

    fn obj_message(&self, obj: usize, prop: i32) -> String {
        data().objects[obj]
            .messages
            .get(&prop)
            .cloned()
            .unwrap_or_default()
    }

    fn rand(&mut self) -> f64 {
        self.rng.random()
    }

    fn choice(&mut self, len: usize) -> usize {
        self.rng.choice_index(len)
    }

    // ----- public command entry ------------------------------------------

    pub fn command(&mut self, raw: &str) -> String {
        self.buf.clear();
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            self.push_line(Line::Echo(trimmed.to_string()));
        }
        self.process(trimmed);
        let out = std::mem::take(&mut self.buf);
        if !out.is_empty() {
            self.push_line(Line::Out(out.clone()));
        }
        out
    }

    fn process(&mut self, raw: &str) {
        let words: Vec<String> = raw.to_lowercase().split_whitespace().map(String::from).collect();

        if self.pending.is_some() {
            let ans = words.first().and_then(|w| match w.as_str() {
                "y" | "yes" => Some(true),
                "n" | "no" => Some(false),
                _ => None,
            });
            match ans {
                Some(a) => {
                    let p = self.pending.take().unwrap();
                    self.pending_casual = false;
                    self.resolve_yesno(p, a);
                    return;
                }
                None => {
                    if self.pending_casual {
                        self.pending = None;
                        self.pending_casual = false;
                        // fall through: process this input as a normal command
                    } else {
                        self.w("Please answer the question.".to_string());
                        return;
                    }
                }
            }
        }

        if words.is_empty() {
            return;
        }
        self.do_command(&words);
    }

    fn resolve_yesno(&mut self, p: Pending, yes: bool) {
        match p {
            Pending::Instructions => self.start2(yes),
            Pending::Resurrect => self.resurrect(yes),
            Pending::Hint(i) => {
                if yes {
                    let m = data().hints[i].message;
                    self.wm(m);
                    self.hint_used[i] = true;
                } else {
                    self.wm(54);
                }
            }
            Pending::AttackDragon => self.kill_dragon(),
            Pending::Quit => {
                self.wm(54);
                if yes {
                    self.score_and_exit();
                }
            }
            Pending::Score => {
                self.wm(54);
                if yes {
                    self.score_and_exit();
                }
            }
            Pending::ReadOyster => {
                if yes {
                    if let Some(i) = self.hint_idx(2) {
                        self.hint_used[i] = true;
                    }
                    self.wm(193);
                } else {
                    self.wm(54);
                }
            }
        }
    }

    fn yesno(&mut self, question: i32, p: Pending, casual: bool) {
        self.wm(question);
        self.pending = Some(p);
        self.pending_casual = casual;
    }

    // ----- aftermath of movement (#2 .. #2605) ----------------------------

    fn move_to(&mut self, newloc: Option<u16>) {
        let loc = self.loc;
        let mut newloc = newloc.unwrap_or(loc);

        if self.is_closing && is_aboveground(newloc) {
            self.wm(130);
            newloc = loc;
            if !self.panic {
                self.clock2 = 15;
                self.panic = true;
            }
        }

        let must_allow_move = newloc == loc
            || self.loc_is_forced()
            || data().rooms[loc as usize].is_forbidden_to_pirate;

        let dwarf_blocking = self
            .dwarves
            .iter()
            .any(|d| d.old_room == newloc && d.has_seen);

        if !must_allow_move && dwarf_blocking {
            newloc = loc;
            self.wm(2);
        }

        self.loc = newloc;
        let loc = newloc;

        let is_dwarf_area =
            !(self.room_is_forced(loc) || data().rooms[loc as usize].is_forbidden_to_pirate);
        if is_dwarf_area && self.dwarf_stage > 0 {
            self.move_dwarves();
        } else {
            if is_dwarf_area && is_after_hall_of_mists(loc) {
                self.dwarf_stage = 1;
            }
            self.describe_location();
        }
    }

    fn move_dwarves(&mut self) {
        if self.dwarf_stage == 1 {
            if is_before_hall_of_mists(self.loc) || self.rand() < 0.95 {
                self.describe_location();
                return;
            }
            self.dwarf_stage = 2;
            for _ in 0..2 {
                if self.rand() < 0.5 && !self.dwarves.is_empty() {
                    let i = self.choice(self.dwarves.len());
                    self.dwarves.remove(i);
                }
            }
            let loc = self.loc;
            for d in self.dwarves.iter_mut() {
                if d.room == loc {
                    *d = Dwarf::new(18, false);
                }
            }
            self.wm(3);
            self.drop_at(AXE, loc);
            self.describe_location();
            return;
        }

        // #6010
        let mut dwarf_count = 0;
        let mut dwarf_attacks = 0;
        let mut knife_wounds = 0;
        let loc = self.loc;

        // Process the five dwarves, then the pirate.
        let count = self.dwarves.len();
        for i in 0..count {
            let mut dw = self.dwarves[i].clone();
            self.step_dwarf(&mut dw);
            // dwarf bookkeeping
            if dw.has_seen {
                dw.room = loc;
                dwarf_count += 1;
                if dw.room == dw.old_room {
                    dwarf_attacks += 1;
                    self.knife_location = Some(loc);
                    if self.rand() < 0.095 * (self.dwarf_stage - 2) as f64 {
                        knife_wounds += 1;
                    }
                }
            }
            self.dwarves[i] = dw;
        }

        // The pirate.
        self.step_pirate();

        // Report what happened.
        if dwarf_count == 1 {
            self.wm(4);
        } else if dwarf_count > 0 {
            self.w(format!(
                "There are {} threatening little dwarves in the room with you.\n",
                dwarf_count
            ));
        }

        if dwarf_attacks > 0 && self.dwarf_stage == 2 {
            self.dwarf_stage = 3;
        }

        let mut k = 0;
        if dwarf_attacks == 1 {
            self.wm(5);
            k = 52;
        } else if dwarf_attacks > 0 {
            self.w(format!("{} of them throw knives at you!\n", dwarf_attacks));
            k = 6;
        }

        if dwarf_attacks == 0 {
            // nothing
        } else if knife_wounds == 0 {
            self.wm(k);
        } else {
            if knife_wounds == 1 {
                self.wm(k + 1);
            } else {
                self.w(format!("{} of them get you!\n", knife_wounds));
            }
            self.oldloc2 = self.loc;
            self.die();
            return;
        }

        self.describe_location();
    }

    /// Move one dwarf (not the pirate) and update its seen/room fields, mirroring
    /// the first half of python's per-actor loop body.
    fn step_dwarf(&mut self, dw: &mut Dwarf) {
        let locs = self.dwarf_candidates(dw.room, dw.old_room);
        let new_room = if !locs.is_empty() {
            locs[self.choice(locs.len())]
        } else {
            dw.old_room
        };
        dw.old_room = dw.room;
        dw.room = new_room;
        if self.loc == dw.room || self.loc == dw.old_room {
            dw.has_seen = true;
        } else if is_before_hall_of_mists(self.loc) {
            dw.has_seen = false;
        }
    }

    fn step_pirate(&mut self) {
        let mut p = self.pirate.clone();
        let locs = self.dwarf_candidates(p.room, p.old_room);
        let new_room = if !locs.is_empty() {
            locs[self.choice(locs.len())]
        } else {
            p.old_room
        };
        p.old_room = p.room;
        p.room = new_room;
        if self.loc == p.room || self.loc == p.old_room {
            p.has_seen = true;
        } else if is_before_hall_of_mists(self.loc) {
            p.has_seen = false;
        }

        if !p.has_seen {
            self.pirate = p;
            return;
        }
        p.room = self.loc;

        // #6020 pirate logic
        if self.loc == CHEST_ROOM || self.obj_prop[CHEST] >= 0 {
            self.pirate = p;
            return;
        }

        let mut carried_treasures: Vec<usize> =
            self.treasures().into_iter().filter(|&t| self.obj_toting[t]).collect();
        if carried_treasures.contains(&PLATINUM) && (self.loc == 100 || self.loc == 101) {
            carried_treasures.retain(|&t| t != PLATINUM);
        }

        if carried_treasures.is_empty() {
            let any_treasure_here = self.treasures().iter().any(|&t| self.is_here(t));
            let one_treasure_left = self.treasures_not_found == self.impossible_treasures + 1;
            let shiver = one_treasure_left
                && !any_treasure_here
                && self.obj_rooms[CHEST].is_empty()
                && self.is_here(LAMP)
                && self.obj_prop[LAMP] == 1;
            if !shiver {
                if p.old_room != p.room && self.rand() < 0.2 {
                    self.wm(127);
                }
                self.pirate = p;
                return;
            }
            self.wm(186);
            self.drop_at(CHEST, CHEST_ROOM);
            self.drop_at(MESSAGE, 140);
        } else {
            // #6022
            self.wm(128);
            if self.obj_rooms[MESSAGE].is_empty() {
                self.drop_at(CHEST, CHEST_ROOM);
            }
            self.drop_at(MESSAGE, 140);
            for t in carried_treasures {
                self.drop_at(t, CHEST_ROOM);
            }
        }

        // #6024
        p.old_room = CHEST_ROOM;
        p.room = CHEST_ROOM;
        p.has_seen = false;
        self.pirate = p;
    }

    fn dwarf_candidates(&self, room: u16, old_room: u16) -> Vec<u16> {
        let d = data();
        let mut set: Vec<u16> = Vec::new();
        for mv in &d.rooms[room as usize].travel {
            if let Action::Room(r) = mv.action {
                // NOTE: python's `can_move` also tests `condition != ('%', 100)`,
                // but `m==100` decodes to `not_dwarf`, never `('%', 100)`, so
                // that exclusion is a vestigial no-op. We replicate the bug:
                // dwarves are *not* actually barred from "forbidden" passages.
                if r >= 15
                    && !d.rooms[r as usize].is_forced()
                    && r != old_room
                    && r != room
                    && !set.contains(&r)
                {
                    set.push(r);
                }
            }
        }
        set.sort_unstable();
        set
    }

    fn describe_location(&mut self) {
        let loc = self.loc;

        if loc == 0 {
            self.die();
            return;
        }

        let could_fall = self.is_dark() && self.could_fall_in_pit;
        if could_fall && !self.loc_is_forced() && self.rand() < 0.35 {
            self.die_here();
            return;
        }

        if self.obj_toting[BEAR] {
            self.wm(141);
        }

        if self.is_dark() && !self.loc_is_forced() {
            self.wm(16);
        } else {
            let period = self.full_description_period;
            let do_short = self.times_described[loc as usize] % period;
            self.times_described[loc as usize] += 1;
            // Write the description straight from the `&'static` data — no clone.
            let room = &data().rooms[loc as usize];
            if do_short != 0 && !room.short.is_empty() {
                self.w_str(&room.short);
            } else {
                self.w_str(&room.long);
            }
        }

        if self.loc_is_forced() {
            let dummy = self.wnum(2);
            self.do_motion(&dummy);
            return;
        }

        if loc == 33 && self.rand() < 0.25 && !self.is_closing {
            self.wm(8);
        }

        if !self.is_dark() {
            for obj in self.objects_here() {
                if obj == STEPS && self.obj_toting[GOLD] {
                    continue;
                }
                if self.obj_prop[obj] < 0 {
                    // Finding a treasure for the first time.
                    if self.is_closed {
                        continue;
                    }
                    self.obj_prop[obj] = if obj == RUG || obj == CHAIN { 1 } else { 0 };
                    self.treasures_not_found -= 1;
                    if self.treasures_not_found > 0
                        && self.treasures_not_found == self.impossible_treasures
                    {
                        self.lamp_turns = self.lamp_turns.min(35);
                    }
                }
                let prop = if obj == STEPS
                    && data().objects[STEPS].start_rooms.get(1) == Some(&self.loc)
                {
                    1
                } else {
                    self.obj_prop[obj]
                };
                let msg = self.obj_message(obj, prop);
                self.w(msg);
            }
        }

        self.finish_turn(None);
    }

    fn say_okay_and_finish(&mut self) {
        self.wm(54);
        self.finish_turn(None);
    }

    fn finish_turn(&mut self, hint_obj: Option<usize>) {
        // Advance RNG so each input affects the future.
        self.rand();

        // Offer a hint if the player has loitered.
        let nhints = data().hints.len();
        for i in 0..nhints {
            let (n, turns_needed, rooms_has_loc) = {
                let h = &data().hints[i];
                (h.n, h.turns_needed, h.rooms.contains(&self.loc))
            };
            if turns_needed == 9999 || self.hint_used[i] {
                continue;
            }
            if rooms_has_loc {
                self.hint_counter[i] += 1;
                if self.hint_counter[i] >= turns_needed {
                    if n != 5 {
                        self.hint_counter[i] = 0;
                    }
                    if self.should_offer_hint(n, hint_obj) {
                        self.hint_counter[i] = 0;
                        let q = data().hints[i].question;
                        self.yesno(q, Pending::Hint(i), false);
                        return;
                    }
                }
            } else {
                self.hint_counter[i] = 0;
            }
        }

        if self.is_closed {
            if self.obj_prop[OYSTER] < 0 && self.obj_toting[OYSTER] {
                let m = self.obj_message(OYSTER, 1);
                self.w(m);
            }
            for obj in self.inventory() {
                if self.obj_prop[obj] < 0 {
                    self.obj_prop[obj] = -1 - self.obj_prop[obj];
                }
            }
        }

        self.could_fall_in_pit = self.is_dark();
        if let Some(k) = self.knife_location {
            if k != self.loc {
                self.knife_location = None;
            }
        }
    }

    fn should_offer_hint(&self, n: i32, hint_obj: Option<usize>) -> bool {
        match n {
            4 => self.obj_prop[GRATE] == 0 && !self.is_here(KEYS),
            5 => self.is_here(BIRD) && self.obj_toting[ROD] && hint_obj == Some(BIRD),
            6 => self.is_here(SNAKE) && !self.is_here(BIRD),
            7 => {
                self.objects_here().is_empty()
                    && self.objects_at(self.oldloc).is_empty()
                    && self.objects_at(self.oldloc2).is_empty()
                    && self.inventory().len() > 1
            }
            8 => self.obj_prop[EMERALD] != 1 && self.obj_prop[PLATINUM] != 1,
            9 => true,
            _ => false,
        }
    }

    // ----- the turn machinery (#2608 .. #19999) ---------------------------

    fn do_command(&mut self, words: &[String]) {
        if self.is_dead {
            self.w("You have gotten yourself killed.".to_string());
            return;
        }

        self.turns += 1;
        if self.treasures_not_found == 0 && self.loc >= 15 && self.loc != 33 {
            self.clock1 -= 1;
            if self.clock1 == 0 {
                self.start_closing_cave();
            }
        }
        if self.clock1 < 0 {
            self.clock2 -= 1;
            if self.clock2 == 0 {
                self.close_cave();
                return;
            }
        }

        if self.obj_prop[LAMP] == 1 {
            self.lamp_turns -= 1;
        }

        if self.lamp_turns <= 30
            && self.is_here(BATTERIES)
            && self.obj_prop[BATTERIES] == 0
            && self.is_here(LAMP)
        {
            self.wm(188);
            self.obj_prop[BATTERIES] = 1;
            if self.obj_toting[BATTERIES] {
                let loc = self.loc;
                self.drop_at(BATTERIES, loc);
            }
            self.lamp_turns += 2500;
            self.warned_about_dim_lamp = false;
        } else if self.lamp_turns == 0 {
            self.lamp_turns = -1;
            self.obj_prop[LAMP] = 0;
            if self.is_here(LAMP) {
                self.wm(184);
            }
        } else if self.lamp_turns < 0 && is_aboveground(self.loc) {
            self.wm(185);
            self.gave_up = true;
            self.score_and_exit();
            return;
        } else if self.lamp_turns <= 30 && !self.warned_about_dim_lamp && self.is_here(LAMP) {
            self.warned_about_dim_lamp = true;
            if self.obj_prop[BATTERIES] == 1 {
                self.wm(189);
            } else if self.obj_rooms[BATTERIES].is_empty() {
                self.wm(183);
            } else {
                self.wm(187);
            }
        }

        self.dispatch_words(words);
    }

    fn word(&self, t: &str) -> Option<W> {
        data().text_to_n.get(t).map(|&n| W {
            n,
            text: t.to_string(),
        })
    }

    fn wnum(&self, n: i32) -> W {
        W {
            n,
            text: data().canonical(n).to_string(),
        }
    }

    fn dispatch_str(&mut self, w: &str) {
        let words = vec![w.to_string()];
        self.dispatch_words(&words);
    }

    fn dispatch_words(&mut self, words: &[String]) {
        if !(1..=2).contains(&words.len()) {
            return self.dont_understand();
        }
        if words[0] == "save" && words.len() > 1 {
            return self.t_suspend();
        }

        let resolved: Vec<Option<W>> = words.iter().map(|w| self.word(w)).collect();
        if resolved.iter().any(|w| w.is_none()) {
            return self.dont_understand();
        }
        let mut word1 = resolved[0].clone().unwrap();
        let mut word2 = if words.len() == 2 {
            Some(resolved[1].clone().unwrap())
        } else {
            None
        };

        // 'enter stream' / 'enter water'
        if self.is(&word1, "enter")
            && word2
                .as_ref()
                .map(|w| self.is(w, "stream") || self.is(w, "water"))
                .unwrap_or(false)
        {
            if self.loc_liquid() == Some(WATER) {
                self.wm(70);
            } else {
                self.wm(43);
            }
            return self.finish_turn(None);
        }

        // #2800 'enter house' -> 'house'
        if (self.is(&word1, "enter") || self.is(&word1, "walk")) && word2.is_some() {
            word1 = word2.take().unwrap();
        }

        // 'water plant' -> 'pour water'
        if (self.is(&word1, "water") || self.is(&word1, "oil"))
            && word2
                .as_ref()
                .map(|w| self.is(w, "plant") || self.is(w, "door"))
                .unwrap_or(false)
        {
            let target = self.referent(word2.as_ref().unwrap().n);
            if self.is_here(target) {
                let pour_n = data().text_to_n["pour"];
                let liquid = word1.clone();
                word1 = W {
                    n: pour_n,
                    text: "pour".to_string(),
                };
                word2 = Some(liquid);
            }
        }

        if self.is(&word1, "say") {
            return match word2 {
                Some(w2) => self.t_say(&word1, &w2),
                None => self.ask_verb_what(&word1),
            };
        }
        if word2.as_ref().map(|w| self.is(w, "say")).unwrap_or(false) {
            let w2 = word2.take().unwrap();
            return self.t_say(&w2, &word1);
        }

        let d = data();
        let k1 = d.kind(word1.n);
        let k2 = word2.as_ref().map(|w| d.kind(w.n));

        // #2630
        if k1 == Kind::Travel && k2.is_none() {
            if word1.text == "west" {
                self.full_wests += 1;
                if self.full_wests == 10 {
                    self.wm(17);
                }
            }
            return self.do_motion(&word1);
        }
        if k1 == Kind::Snappy && k2.is_none() {
            let n = word1.n % 1000;
            self.wm(n);
            return self.finish_turn(None);
        }

        let (verb, noun): (Option<W>, Option<W>) = match (k1, k2) {
            (Kind::Noun, None) => (None, Some(word1.clone())),
            (Kind::Verb, None) => (Some(word1.clone()), None),
            (Kind::Verb, Some(Kind::Noun)) => (Some(word1.clone()), word2.clone()),
            (Kind::Noun, Some(Kind::Verb)) => (word2.clone(), Some(word1.clone())),
            _ => return self.dont_understand(),
        };

        let mut obj: Option<usize> = None;
        if let Some(noun) = &noun {
            let mut o = self.referent(noun.n);
            let mut obj_here = self.is_here(o);
            if !obj_here {
                if o == GRATE {
                    if matches!(self.loc, 1 | 4 | 7) {
                        return self.dispatch_str("depression");
                    } else if (10..15).contains(&self.loc) {
                        return self.dispatch_str("entrance");
                    }
                } else if self.is(noun, "dwarf") {
                    obj_here = self.dwarf_at_loc();
                } else if (Some(o) == self.bottle_contents && self.is_here(BOTTLE))
                    || Some(o) == self.loc_liquid()
                {
                    obj_here = true;
                } else if o == PLANT && self.is_here(PLANT2) && self.obj_prop[PLANT2] != 0 {
                    o = PLANT2;
                    obj_here = true;
                } else if o == KNIFE && self.knife_location == Some(self.loc) {
                    self.knife_location = None;
                    self.wm(116);
                    return self.finish_turn(None);
                } else if o == ROD && self.is_here(ROD2) {
                    o = ROD2;
                    obj_here = true;
                } else if verb
                    .as_ref()
                    .map(|v| self.is(v, "find") || self.is(v, "inventory"))
                    .unwrap_or(false)
                {
                    obj_here = true;
                }
            }
            if !obj_here {
                return self.i_see_no(&noun.text);
            }
            if verb.is_none() {
                self.w(format!("What do you want to do with the {}?\n", noun.text));
                return self.finish_turn(None);
            }
            obj = Some(o);
        }

        let verb = verb.unwrap();
        let vn = data().canonical(verb.n).to_string();
        match obj {
            Some(o) => self.call_t(&vn, &verb, o),
            None => self.call_i(&vn, &verb),
        }
    }

    fn dont_understand(&mut self) {
        let n = self.rand();
        if n < 0.20 {
            self.wm(61);
        } else if n < 0.36 {
            self.wm(13);
        } else {
            self.wm(60);
        }
        self.finish_turn(None);
    }

    fn i_see_no(&mut self, thing: &str) {
        self.w(format!("I see no {} here.\n", thing));
        self.finish_turn(None);
    }

    // ----- motion (#8) ----------------------------------------------------

    fn do_motion(&mut self, word: &W) {
        if self.is(word, "null") {
            self.move_to(None);
            return;
        } else if self.is(word, "back") {
            return self.motion_back(word);
        } else if self.is(word, "look") {
            if self.look_complaints > 0 {
                self.wm(15);
                self.look_complaints -= 1;
            }
            self.times_described[self.loc as usize] = 0;
            self.move_to(None);
            self.could_fall_in_pit = false;
            return;
        } else if self.is(word, "cave") {
            if is_aboveground(self.loc) {
                self.wm(57);
            } else {
                self.wm(58);
            }
            self.move_to(None);
            return;
        }

        self.oldloc2 = self.oldloc;
        self.oldloc = self.loc;
        self.travel(word);
    }

    fn motion_back(&mut self, _word: &W) {
        let dest = if self.room_is_forced(self.oldloc) {
            self.oldloc2
        } else {
            self.oldloc
        };
        self.oldloc2 = self.oldloc;
        self.oldloc = self.loc;
        if dest == self.loc {
            self.wm(91);
            self.move_to(None);
            return;
        }
        let d = data();
        let mut chosen: Option<W> = None;
        let mut alt: Option<W> = None;
        for mv in &d.rooms[self.loc as usize].travel {
            if let Action::Room(r) = mv.action {
                if r == dest {
                    if let Some(&v) = mv.verbs.first() {
                        chosen = Some(self.wnum(v));
                    }
                    break;
                } else if self.room_is_forced(r) {
                    if let Some(Action::Room(r2)) = d.rooms[r as usize].travel.first().map(|m| m.action.clone()) {
                        if r2 == dest {
                            if let Some(&v) = mv.verbs.first() {
                                alt = Some(self.wnum(v));
                            }
                        }
                    }
                }
            }
        }
        let word = match chosen.or(alt) {
            Some(w) => w,
            None => {
                self.wm(140);
                self.move_to(None);
                return;
            }
        };
        self.travel(&word);
    }

    /// The travel-table walk (the loop body shared by `do_motion` and `back`).
    fn travel(&mut self, word: &W) {
        let d = data();
        let travel = &d.rooms[self.loc as usize].travel;
        for idx in 0..travel.len() {
            let mv = &d.rooms[self.loc as usize].travel[idx];
            if !(mv.is_forced || mv.verbs.contains(&word.n)) {
                continue;
            }
            let allowed = match &mv.condition {
                Condition::Always | Condition::NotDwarf => true,
                Condition::Percent(p) => 100.0 * self.rand() < *p as f64,
                Condition::Carrying(o) => self.obj_toting[*o],
                Condition::CarryingOrWith(o) => self.is_here(*o),
                Condition::PropNot(o, v) => self.obj_prop[*o] != *v,
            };
            if !allowed {
                continue;
            }
            let action = d.rooms[self.loc as usize].travel[idx].action.clone();
            match action {
                Action::Room(r) => {
                    self.move_to(Some(r));
                    return;
                }
                Action::Message(m) => {
                    self.wm(m);
                    self.move_to(None);
                    return;
                }
                Action::Special(301) => return self.special_plover(),
                Action::Special(302) => {
                    let loc = self.loc;
                    self.drop_at(EMERALD, loc);
                    return self.do_motion(word);
                }
                Action::Special(303) => return self.special_troll(),
                Action::Special(_) => {}
            }
        }

        // #50 — no exit that way.
        let n = word.n;
        if (29..=30).contains(&n) || (43..=50).contains(&n) {
            self.wm(9);
        } else if matches!(n, 7 | 36 | 37) {
            self.wm(10);
        } else if matches!(n, 11 | 19) {
            self.wm(11);
        } else if matches!(n, 62 | 65) {
            self.wm(42);
        } else if n == 17 {
            self.wm(80);
        } else {
            self.wm(12);
        }
        self.move_to(None);
    }

    fn special_plover(&mut self) {
        // #30100
        let inv = self.inventory();
        if !inv.is_empty() && inv != vec![EMERALD] {
            self.wm(117);
            self.move_to(None);
        } else if self.loc == 100 {
            self.move_to(Some(99));
        } else {
            self.move_to(Some(100));
        }
    }

    fn special_troll(&mut self) {
        // #30300
        if self.obj_prop[TROLL] == 1 {
            let m = self.obj_message(TROLL, 1);
            self.w(m);
            self.obj_prop[TROLL] = 0;
            self.obj_rooms[TROLL] = data().objects[TROLL].start_rooms.clone();
            self.hide(TROLL2);
            self.move_to(None);
            return;
        }
        let mut places = data().objects[TROLL].start_rooms.clone();
        places.retain(|&r| r != self.loc);
        self.loc = places[0];
        if self.obj_prop[TROLL] == 0 {
            self.obj_prop[TROLL] = 1;
        }
        if !self.obj_toting[BEAR] {
            self.move_to(None);
            return;
        }
        self.wm(162);
        self.obj_prop[CHASM] = 1;
        self.obj_prop[TROLL] = 2;
        let loc = self.loc;
        self.drop_at(BEAR, loc);
        self.obj_fixed[BEAR] = 1;
        self.obj_prop[BEAR] = 3;
        if self.obj_prop[SPICES] < 0 {
            self.impossible_treasures += 1;
        }
        self.oldloc2 = self.loc;
        self.die();
    }

    // ----- death & reincarnation (#90, #99) -------------------------------

    fn die_here(&mut self) {
        self.wm(23);
        self.oldloc2 = self.loc;
        self.die();
    }

    fn die(&mut self) {
        self.deaths += 1;
        self.is_dead = true;
        if self.is_closing {
            self.wm(131);
            self.score_and_exit();
            return;
        }
        let q = 79 + self.deaths * 2;
        self.yesno(q, Pending::Resurrect, false);
    }

    fn resurrect(&mut self, yes: bool) {
        if yes {
            let m = 80 + self.deaths * 2;
            self.wm(m);
            if self.deaths < self.max_deaths {
                if let Some(c) = self.bottle_contents {
                    self.hide(c);
                }
                self.is_dead = false;
                if self.obj_toting[LAMP] {
                    self.obj_prop[LAMP] = 0;
                }
                let dest = self.oldloc2;
                for obj in self.inventory() {
                    if obj == LAMP {
                        self.drop_at(LAMP, 1);
                    } else {
                        self.drop_at(obj, dest);
                    }
                }
                self.loc = 3;
                self.describe_location();
                return;
            }
        } else {
            self.wm(54);
        }
        self.score_and_exit();
    }

    // ----- verb dispatch tables -------------------------------------------

    fn call_i(&mut self, vn: &str, verb: &W) {
        match vn {
            "carry" => self.i_carry(verb),
            "drop" => self.ask_verb_what(verb),
            "say" => self.ask_verb_what(verb),
            "unlock" | "lock" => self.i_unlock(verb),
            "nothing" => self.say_okay_and_finish(),
            "light" => self.t_light(verb),
            "extinguish" => self.t_extinguish(verb),
            "wave" => self.ask_verb_what(verb),
            "calm" => self.ask_verb_what(verb),
            "walk" => self.ask_verb_what(verb),
            "attack" => self.i_attack(verb),
            "pour" => self.i_pour(verb),
            "eat" => self.i_eat(verb),
            "drink" => self.i_drink(verb),
            "rub" => self.ask_verb_what(verb),
            "throw" => self.ask_verb_what(verb),
            "quit" => self.i_quit(verb),
            "find" => self.ask_verb_what(verb),
            "inventory" => self.i_inventory(verb),
            "feed" => self.ask_verb_what(verb),
            "fill" => self.i_fill(verb),
            "blast" => self.t_blast(verb),
            "score" => self.i_score(verb),
            "fee" => self.i_fee(verb),
            "brief" => self.i_brief(verb),
            "read" => self.i_read(verb),
            "break" => self.ask_verb_what(verb),
            "wake" => self.ask_verb_what(verb),
            "suspend" => self.i_suspend(verb),
            "hours" => self.i_hours(verb),
            _ => self.write_default_message(verb),
        }
    }

    fn call_t(&mut self, vn: &str, verb: &W, obj: usize) {
        match vn {
            "carry" => self.t_carry(verb, obj),
            "drop" => self.t_drop(verb, obj),
            "say" => self.t_say(verb, &self.wnum_for_obj(obj)),
            "unlock" | "lock" => self.t_unlock(verb, obj),
            "nothing" => self.say_okay_and_finish(),
            "light" => self.t_light(verb),
            "extinguish" => self.t_extinguish(verb),
            "wave" => self.t_wave(verb, obj),
            "calm" => self.write_default_message(verb),
            "walk" => self.write_default_message(verb),
            "attack" => self.t_attack(verb, Some(obj)),
            "pour" => self.t_pour(verb, obj),
            "eat" => self.t_eat(verb, obj),
            "drink" => self.t_drink(verb, obj),
            "rub" => self.t_rub(verb, obj),
            "throw" => self.t_throw(verb, obj),
            "quit" => self.write_default_message(verb),
            "find" => self.t_find(verb, obj),
            "inventory" => self.t_find(verb, obj),
            "feed" => self.t_feed(verb, obj),
            "fill" => self.t_fill(verb, obj),
            "blast" => self.t_blast(verb),
            "score" => self.write_default_message(verb),
            "fee" => self.write_default_message(verb),
            "brief" => self.write_default_message(verb),
            "read" => self.t_read(verb, obj),
            "break" => self.t_break(verb, obj),
            "wake" => self.t_wake(verb, obj),
            "suspend" => self.t_suspend(),
            "hours" => self.write_default_message(verb),
            _ => self.write_default_message(verb),
        }
    }

    fn wnum_for_obj(&self, _obj: usize) -> W {
        // t_say is only reached with a noun via "say <noun>"; we reconstruct a
        // word so it can echo. Unused path in practice.
        W {
            n: 0,
            text: String::new(),
        }
    }

    fn ask_verb_what(&mut self, verb: &W) {
        self.w(format!("{} What?\n", verb.text));
        self.finish_turn(None);
    }

    fn write_default_message(&mut self, verb: &W) {
        let m = self.default_msg(verb);
        self.w(m);
        self.finish_turn(None);
    }

    // ----- verbs ----------------------------------------------------------

    fn i_carry(&mut self, verb: &W) {
        let objs = self.objects_here();
        if objs.len() != 1 || self.dwarf_at_loc() {
            self.ask_verb_what(verb);
        } else {
            self.t_carry(verb, objs[0]);
        }
    }

    fn t_carry(&mut self, verb: &W, obj: usize) {
        if self.obj_toting[obj] {
            let m = self.default_msg(verb);
            self.w(m);
            return self.finish_turn(None);
        }
        if self.obj_fixed[obj] != 0 || self.obj_rooms[obj].len() > 1 {
            if obj == PLANT && self.obj_prop[obj] <= 0 {
                self.wm(115);
            } else if obj == BEAR && self.obj_prop[BEAR] == 1 {
                self.wm(169);
            } else if obj == CHAIN && self.obj_prop[CHAIN] != 0 {
                self.wm(170);
            } else {
                self.wm(25);
            }
            return self.finish_turn(None);
        }
        let mut obj = obj;
        if obj == WATER || obj == OIL {
            if self.is_here(BOTTLE) && self.bottle_contents == Some(obj) {
                obj = BOTTLE;
            } else {
                if !self.obj_toting[BOTTLE] {
                    self.wm(104);
                } else if self.bottle_contents.is_some() {
                    self.wm(105);
                } else {
                    return self.t_fill(verb, BOTTLE);
                }
                return self.finish_turn(None);
            }
        }
        if self.inventory().len() >= 7 {
            self.wm(92);
            return self.finish_turn(None);
        }
        if obj == BIRD && self.obj_prop[BIRD] == 0 {
            if self.obj_toting[ROD] {
                self.wm(26);
                return self.finish_turn(Some(BIRD));
            }
            if !self.obj_toting[CAGE] {
                self.wm(27);
                return self.finish_turn(None);
            }
            self.obj_prop[BIRD] = 1;
        }
        if (obj == BIRD || obj == CAGE) && self.obj_prop[BIRD] != 0 {
            self.carry(BIRD);
            self.carry(CAGE);
        } else {
            self.carry(obj);
            if obj == BOTTLE && self.bottle_contents.is_some() {
                let c = self.bottle_contents.unwrap();
                self.carry(c);
            }
        }
        self.say_okay_and_finish();
    }

    fn t_drop(&mut self, verb: &W, obj: usize) {
        let mut obj = obj;
        if obj == ROD && !self.obj_toting[ROD] && self.obj_toting[ROD2] {
            obj = ROD2;
        }
        if !self.obj_toting[obj] {
            let m = self.default_msg(verb);
            self.w(m);
            return self.finish_turn(None);
        }

        if obj == BIRD && self.is_here(SNAKE) {
            self.wm(30);
            if self.is_closed {
                return self.wake_repository_dwarves();
            }
            self.obj_prop[SNAKE] = 1;
            self.hide(SNAKE);
        } else if obj == COINS && self.is_here(MACHINE) {
            self.hide(COINS);
            let loc = self.loc;
            self.drop_at(BATTERIES, loc);
            let m = self.obj_message(BATTERIES, 0);
            self.w(m);
            return self.finish_turn(None);
        } else if obj == BIRD && self.is_here(DRAGON) && self.obj_prop[DRAGON] == 0 {
            self.wm(154);
            self.hide(BIRD);
            self.obj_prop[BIRD] = 0;
            if !self.obj_rooms[SNAKE].is_empty() {
                self.impossible_treasures += 1;
            }
            return self.finish_turn(None);
        } else if obj == BEAR && self.is_here(TROLL) {
            self.wm(163);
            self.hide(TROLL);
            self.obj_rooms[TROLL2] = data().objects[TROLL].start_rooms.clone();
            self.obj_prop[TROLL] = 2;
        } else if obj == VASE && self.loc != 96 {
            if self.is_at(PILLOW, self.loc) {
                self.obj_prop[VASE] = 0;
            } else {
                self.obj_prop[VASE] = 2;
                self.obj_fixed[VASE] = 1;
            }
            let p = self.obj_prop[VASE];
            let m = self.obj_message(VASE, p + 1);
            self.w(m);
        } else {
            self.wm(54);
        }

        // #9021
        if Some(obj) == self.bottle_contents {
            obj = BOTTLE;
        }
        if obj == BOTTLE {
            if let Some(c) = self.bottle_contents {
                self.hide(c);
            }
        }
        if obj == CAGE && self.obj_prop[BIRD] != 0 {
            let loc = self.loc;
            self.drop_at(BIRD, loc);
        } else if obj == BIRD {
            self.obj_prop[BIRD] = 0;
        }
        let loc = self.loc;
        self.drop_at(obj, loc);
        self.finish_turn(None);
    }

    fn t_say(&mut self, _verb: &W, word: &W) {
        if matches!(word.n, 62 | 65 | 71 | 2025) {
            self.dispatch_str(&word.text);
        } else {
            self.w(format!("Okay, \"{}\".", word.text));
            self.finish_turn(None);
        }
    }

    fn i_unlock(&mut self, verb: &W) {
        let objs: Vec<usize> = [GRATE, DOOR, OYSTER, CLAM, CHAIN]
            .into_iter()
            .filter(|&o| self.is_here(o))
            .collect();
        if objs.len() > 1 {
            self.ask_verb_what(verb);
        } else if objs.len() == 1 {
            self.t_unlock(verb, objs[0]);
        } else {
            self.wm(28);
            self.finish_turn(None);
        }
    }

    fn t_unlock(&mut self, verb: &W, obj: usize) {
        if obj == CLAM || obj == OYSTER {
            let oy = if obj == OYSTER { 1 } else { 0 };
            if self.is(verb, "lock") {
                self.wm(61);
            } else if !self.obj_toting[TRIDENT] {
                self.wm(122 + oy);
            } else if self.obj_toting[obj] {
                self.wm(120 + oy);
            } else if obj == OYSTER {
                self.wm(125);
            } else {
                self.wm(124);
                self.hide(CLAM);
                let loc = self.loc;
                self.drop_at(OYSTER, loc);
                self.drop_at(PEARL, 105);
            }
        } else if obj == DOOR {
            if self.obj_prop[DOOR] == 1 {
                self.wm(54);
            } else {
                self.wm(111);
            }
        } else if obj == CAGE {
            self.wm(32);
        } else if obj == KEYS {
            self.wm(55);
        } else if obj == GRATE || obj == CHAIN {
            if !self.is_here(KEYS) {
                self.wm(31);
            } else if obj == CHAIN {
                self.unlock_chain(verb);
            } else if self.is_closing {
                if !self.panic {
                    self.clock2 = 15;
                    self.panic = true;
                }
                self.wm(130);
            } else {
                let oldprop = self.obj_prop[GRATE];
                self.obj_prop[GRATE] = if self.is(verb, "lock") { 0 } else { 1 };
                self.wm(34 + oldprop + 2 * self.obj_prop[GRATE]);
            }
        } else {
            let m = self.default_msg(verb);
            self.w(m);
        }
        self.finish_turn(None);
    }

    fn unlock_chain(&mut self, verb: &W) {
        if self.is(verb, "unlock") {
            if self.obj_prop[CHAIN] == 0 {
                self.wm(37);
            } else if self.obj_prop[BEAR] == 0 {
                self.wm(41);
            } else {
                self.obj_prop[CHAIN] = 0;
                self.obj_fixed[CHAIN] = 0;
                if self.obj_prop[BEAR] != 3 {
                    self.obj_prop[BEAR] = 2;
                }
                self.obj_fixed[BEAR] = 2 - self.obj_prop[BEAR];
                self.wm(171);
            }
        } else if !data().objects[CHAIN].start_rooms.contains(&self.loc) {
            self.wm(173);
        } else if self.obj_prop[CHAIN] != 0 {
            self.wm(34);
        } else {
            self.obj_prop[CHAIN] = 2;
            if self.obj_toting[CHAIN] {
                let loc = self.loc;
                self.drop_at(CHAIN, loc);
            }
            self.obj_fixed[CHAIN] = 1;
            self.wm(172);
        }
    }

    fn t_light(&mut self, verb: &W) {
        if !self.is_here(LAMP) {
            let m = self.default_msg(verb);
            self.w(m);
        } else if self.lamp_turns <= 0 {
            self.wm(184);
        } else {
            self.obj_prop[LAMP] = 1;
            self.wm(39);
            if !data().rooms[self.loc as usize].is_light {
                return self.describe_location();
            }
        }
        self.finish_turn(None);
    }

    fn t_extinguish(&mut self, verb: &W) {
        if !self.is_here(LAMP) {
            let m = self.default_msg(verb);
            self.w(m);
        } else {
            self.obj_prop[LAMP] = 0;
            self.wm(40);
            if !data().rooms[self.loc as usize].is_light {
                self.wm(16);
            }
        }
        self.finish_turn(None);
    }

    fn t_wave(&mut self, verb: &W, obj: usize) {
        if obj == ROD
            && self.obj_toting[ROD]
            && self.is_here(FISSURE)
            && !self.is_closing
        {
            self.obj_prop[FISSURE] = if self.obj_prop[FISSURE] != 0 { 0 } else { 1 };
            let p = self.obj_prop[FISSURE];
            let m = self.obj_message(FISSURE, 2 - p);
            self.w(m);
        } else if self.obj_toting[obj] || (obj == ROD && self.obj_toting[ROD2]) {
            let m = self.default_msg(verb);
            self.w(m);
        } else {
            self.wm(29);
        }
        self.finish_turn(None);
    }

    fn i_attack(&mut self, verb: &W) {
        let mut enemies = vec![SNAKE, DRAGON, TROLL, BEAR];
        if self.dwarf_stage >= 2 {
            // Treat a dwarf in the room as an enemy.
            if self.dwarf_at_loc() {
                enemies.push(DWARF);
            }
        }
        let dangers: Vec<usize> = enemies
            .into_iter()
            .filter(|&o| if o == DWARF { self.dwarf_at_loc() } else { self.is_here(o) })
            .collect();
        if dangers.len() > 1 {
            return self.ask_verb_what(verb);
        }
        if dangers.len() == 1 {
            return self.t_attack(verb, Some(dangers[0]));
        }
        let mut targets = vec![];
        if self.is_here(BIRD) && !self.is(verb, "throw") {
            targets.push(BIRD);
        }
        if self.is_here(CLAM) || self.is_here(OYSTER) {
            targets.push(CLAM);
        }
        if targets.len() > 1 {
            self.ask_verb_what(verb)
        } else if targets.len() == 1 {
            self.t_attack(verb, Some(targets[0]))
        } else {
            self.t_attack(verb, None)
        }
    }

    fn t_attack(&mut self, _verb: &W, obj: Option<usize>) {
        match obj {
            Some(BIRD) => {
                if self.is_closed {
                    self.wm(137);
                } else {
                    self.hide(BIRD);
                    self.obj_prop[BIRD] = 0;
                    if !self.obj_rooms[SNAKE].is_empty() {
                        self.impossible_treasures += 1;
                    }
                    self.wm(45);
                }
            }
            Some(CLAM) | Some(OYSTER) => self.wm(150),
            Some(SNAKE) => self.wm(46),
            Some(DWARF) => {
                if self.is_closed {
                    return self.wake_repository_dwarves();
                }
                self.wm(49);
            }
            Some(DRAGON) => {
                if self.obj_prop[DRAGON] != 0 {
                    self.wm(167);
                } else {
                    return self.yesno(49, Pending::AttackDragon, true);
                }
            }
            Some(TROLL) => self.wm(157),
            Some(BEAR) => {
                let m = 165 + (self.obj_prop[BEAR] + 1) / 2;
                self.wm(m);
            }
            _ => self.wm(44),
        }
        self.finish_turn(None);
    }

    fn kill_dragon(&mut self) {
        let m = self.obj_message(DRAGON, 1);
        self.w(m);
        self.obj_prop[DRAGON] = 2;
        self.obj_fixed[DRAGON] = 1;
        let r0 = self.obj_rooms[DRAGON][0];
        let r1 = self.obj_rooms[DRAGON][1];
        let newroom = (r0 + r1) / 2;
        self.drop_at(DRAGON, newroom);
        self.obj_prop[RUG] = 0;
        self.obj_fixed[RUG] = 0;
        self.drop_at(RUG, newroom);
        for old in [r0, r1] {
            for o in self.objects_at(old) {
                self.drop_at(o, newroom);
            }
        }
        self.move_to(Some(newroom));
    }

    fn i_pour(&mut self, verb: &W) {
        match self.bottle_contents {
            None => self.ask_verb_what(verb),
            Some(c) => self.t_pour(verb, c),
        }
    }

    fn t_pour(&mut self, verb: &W, obj: usize) {
        if obj == BOTTLE {
            return self.i_pour(verb);
        }
        if !self.obj_toting[obj] {
            let m = self.default_msg(verb);
            self.w(m);
        } else if obj != OIL && obj != WATER {
            self.wm(78);
        } else {
            self.obj_prop[BOTTLE] = 1;
            self.bottle_contents = None;
            self.hide(obj);
            if self.is_here(PLANT) {
                if obj != WATER {
                    self.wm(112);
                } else {
                    let p = self.obj_prop[PLANT];
                    let m = self.obj_message(PLANT, p + 1);
                    self.w(m);
                    self.obj_prop[PLANT] = (self.obj_prop[PLANT] + 2) % 6;
                    self.obj_prop[PLANT2] = self.obj_prop[PLANT] / 2;
                    return self.move_to(None);
                }
            } else if self.is_here(DOOR) {
                self.obj_prop[DOOR] = if obj == OIL { 1 } else { 0 };
                self.wm(113 + self.obj_prop[DOOR]);
            } else {
                self.wm(77);
            }
        }
        self.finish_turn(None);
    }

    fn i_eat(&mut self, verb: &W) {
        if self.is_here(FOOD) {
            self.t_eat(verb, FOOD);
        } else {
            self.ask_verb_what(verb);
        }
    }

    fn t_eat(&mut self, verb: &W, obj: usize) {
        if obj == FOOD {
            self.hide(FOOD);
            self.wm(72);
        } else if matches!(obj, BIRD | SNAKE | CLAM | OYSTER | DWARF | DRAGON | TROLL | BEAR) {
            self.wm(71);
        } else {
            let m = self.default_msg(verb);
            self.w(m);
        }
        self.finish_turn(None);
    }

    fn i_drink(&mut self, verb: &W) {
        if self.is_here(WATER) || self.loc_liquid() == Some(WATER) {
            self.t_drink(verb, WATER);
        } else {
            self.ask_verb_what(verb);
        }
    }

    fn t_drink(&mut self, verb: &W, obj: usize) {
        if obj != WATER {
            self.wm(110);
        } else if self.is_here(WATER) {
            self.obj_prop[BOTTLE] = 1;
            self.bottle_contents = None;
            self.hide(WATER);
            self.wm(74);
        } else if self.loc_liquid() == Some(WATER) {
            let m = self.default_msg(verb);
            self.w(m);
        }
        self.finish_turn(None);
    }

    fn t_rub(&mut self, verb: &W, obj: usize) {
        if obj == LAMP {
            let m = self.default_msg(verb);
            self.w(m);
        } else {
            self.wm(71);
        }
        self.finish_turn(None);
    }

    fn t_throw(&mut self, verb: &W, obj: usize) {
        let mut obj = obj;
        if obj == ROD && !self.obj_toting[ROD] && self.obj_toting[ROD2] {
            obj = ROD2;
        }
        if !self.obj_toting[obj] {
            let m = self.default_msg(verb);
            self.w(m);
            return self.finish_turn(None);
        }
        if data().objects[obj].is_treasure && self.is_here(TROLL) {
            self.wm(159);
            self.hide(obj);
            self.hide(TROLL);
            self.obj_rooms[TROLL2] = data().objects[TROLL].start_rooms.clone();
            return self.finish_turn(None);
        }
        if obj == FOOD && self.is_here(BEAR) {
            return self.t_feed(verb, BEAR);
        }
        if obj != AXE {
            return self.t_drop(verb, obj);
        }

        // Throwing the axe.
        let dwarves_here: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| self.dwarves[i].room == self.loc)
            .collect();
        if !dwarves_here.is_empty() {
            // 1/3 chance to kill.
            if self.choice(3) == 0 {
                self.dwarves.remove(dwarves_here[0]);
                self.dwarves_killed += 1;
                if self.dwarves_killed == 1 {
                    self.wm(149);
                } else {
                    self.wm(47);
                }
            } else {
                self.wm(48);
            }
            let loc = self.loc;
            self.drop_at(AXE, loc);
            let null = self.wnum_null();
            return self.do_motion(&null);
        }
        if self.is_here(DRAGON) && self.obj_prop[DRAGON] == 0 {
            self.wm(152);
            let loc = self.loc;
            self.drop_at(AXE, loc);
            let null = self.wnum_null();
            return self.do_motion(&null);
        }
        if self.is_here(TROLL) {
            self.wm(158);
            let loc = self.loc;
            self.drop_at(AXE, loc);
            let null = self.wnum_null();
            return self.do_motion(&null);
        }
        if self.is_here(BEAR) && self.obj_prop[BEAR] == 0 {
            self.wm(164);
            let loc = self.loc;
            self.drop_at(AXE, loc);
            self.obj_fixed[AXE] = 1;
            self.obj_prop[AXE] = 1;
            return self.finish_turn(None);
        }
        self.t_attack(verb, None);
    }

    fn wnum_null(&self) -> W {
        self.word("null").unwrap_or(W {
            n: 21,
            text: "null".to_string(),
        })
    }

    fn i_quit(&mut self, _verb: &W) {
        self.yesno(22, Pending::Quit, false);
    }

    fn t_find(&mut self, verb: &W, obj: usize) {
        if self.obj_toting[obj] {
            self.wm(24);
        } else if self.is_closed {
            self.wm(138);
        } else if self.is_here(obj)
            || self.loc_liquid() == Some(obj)
            || (obj == DWARF && self.dwarf_at_loc())
        {
            self.wm(94);
        } else {
            let m = self.default_msg(verb);
            self.w(m);
        }
        self.finish_turn(None);
    }

    fn i_inventory(&mut self, _verb: &W) {
        let objs: Vec<usize> = self.inventory().into_iter().filter(|&o| o != BEAR).collect();
        let mut first = true;
        for obj in &objs {
            if first {
                self.wm(99);
                first = false;
            }
            let m = data().objects[*obj].inventory_message.clone();
            self.w(m);
        }
        if self.obj_toting[BEAR] {
            self.wm(141);
        }
        if objs.is_empty() {
            self.wm(98);
        }
        self.finish_turn(None);
    }

    fn t_feed(&mut self, verb: &W, obj: usize) {
        match obj {
            BIRD => self.wm(100),
            TROLL => self.wm(182),
            DRAGON => {
                if self.obj_prop[DRAGON] != 0 {
                    self.wm(110);
                } else {
                    self.wm(102);
                }
            }
            SNAKE => {
                if self.is_closed || !self.is_here(BIRD) {
                    self.wm(102);
                } else {
                    self.wm(101);
                    self.hide(BIRD);
                    self.obj_prop[BIRD] = 0;
                    self.impossible_treasures += 1;
                }
            }
            DWARF => {
                if self.is_here(FOOD) {
                    self.wm(103);
                    self.dwarf_stage += 1;
                } else {
                    let m = self.default_msg(verb);
                    self.w(m);
                }
            }
            BEAR => {
                if !self.is_here(FOOD) {
                    if self.obj_prop[BEAR] == 0 {
                        self.wm(102);
                    } else if self.obj_prop[BEAR] == 3 {
                        self.wm(110);
                    } else {
                        let m = self.default_msg(verb);
                        self.w(m);
                    }
                } else {
                    self.hide(FOOD);
                    self.obj_prop[BEAR] = 1;
                    self.obj_fixed[AXE] = 0;
                    self.obj_prop[AXE] = 0;
                    self.wm(168);
                }
            }
            _ => self.wm(14),
        }
        self.finish_turn(None);
    }

    fn i_fill(&mut self, verb: &W) {
        if self.is_here(BOTTLE) {
            self.t_fill(verb, BOTTLE);
        } else {
            self.ask_verb_what(verb);
        }
    }

    fn t_fill(&mut self, verb: &W, obj: usize) {
        if obj == BOTTLE {
            match self.loc_liquid() {
                None => self.wm(106),
                Some(_) if self.bottle_contents.is_some() => self.wm(105),
                Some(liquid) => {
                    self.bottle_contents = Some(liquid);
                    self.obj_prop[BOTTLE] = if liquid == WATER { 0 } else { 2 };
                    if self.obj_toting[BOTTLE] {
                        self.obj_toting[liquid] = true;
                    }
                    if liquid == OIL {
                        self.wm(108);
                    } else {
                        self.wm(107);
                    }
                }
            }
        } else if obj == VASE {
            if self.obj_toting[VASE] {
                if self.loc_liquid().is_none() {
                    self.wm(144);
                } else {
                    self.wm(145);
                    let loc = self.loc;
                    self.drop_at(VASE, loc);
                    self.obj_prop[VASE] = 2;
                    self.obj_fixed[VASE] = 1;
                }
            } else {
                let m = self.default_msg(verb);
                self.w(m);
            }
        } else {
            let m = self.default_msg(verb);
            self.w(m);
        }
        self.finish_turn(None);
    }

    fn t_blast(&mut self, verb: &W) {
        if self.obj_prop[ROD2] < 0 || !self.is_closed {
            let m = self.default_msg(verb);
            self.w(m);
            return self.finish_turn(None);
        }
        if self.is_here(ROD2) {
            self.bonus = 135;
        } else if self.loc == 115 {
            self.bonus = 134;
        } else {
            self.bonus = 133;
        }
        let b = self.bonus;
        self.wm(b);
        self.score_and_exit();
    }

    fn i_score(&mut self, _verb: &W) {
        let (score, max_score) = self.compute_score(true);
        self.w(format!(
            "If you were to quit now, you would score {} out of a possible {}.\n",
            score, max_score
        ));
        self.yesno(143, Pending::Score, false);
    }

    fn i_fee(&mut self, verb: &W) {
        let group = data().groups.get(&verb.n).cloned().unwrap_or_default();
        let n = group.iter().position(|t| *t == verb.text).unwrap_or(0) as i32;
        if n == 0 {
            self.foobar = self.turns;
            self.wm(54);
        } else if n != self.turns - self.foobar {
            self.wm(151);
        } else if n < 3 {
            self.wm(54);
        } else {
            self.foobar = -1;
            let start = data().objects[EGGS].start_rooms[0];
            if self.is_at(EGGS, start) || (self.obj_toting[EGGS] && self.loc == start) {
                self.wm(54);
            } else {
                if self.obj_rooms[EGGS].is_empty()
                    && self.obj_rooms[TROLL].is_empty()
                    && self.obj_prop[TROLL] == 0
                {
                    self.obj_prop[TROLL] = 1;
                }
                if self.loc == start {
                    let m = self.obj_message(EGGS, 0);
                    self.w(m);
                } else if self.is_here(EGGS) {
                    let m = self.obj_message(EGGS, 1);
                    self.w(m);
                } else {
                    let m = self.obj_message(EGGS, 2);
                    self.w(m);
                }
                self.obj_rooms[EGGS] = data().objects[EGGS].start_rooms.clone();
                self.obj_toting[EGGS] = false;
            }
        }
        self.finish_turn(None);
    }

    fn i_brief(&mut self, _verb: &W) {
        self.wm(156);
        self.full_description_period = 10000;
        self.look_complaints = 0;
        self.finish_turn(None);
    }

    fn i_read(&mut self, verb: &W) {
        if self.is_closed && self.obj_toting[OYSTER] {
            return self.t_read(verb, OYSTER);
        }
        let objs: Vec<usize> = [MAGAZINE, TABLET, MESSAGE]
            .into_iter()
            .filter(|&o| self.is_here(o))
            .collect();
        if objs.len() != 1 || self.is_dark() {
            self.ask_verb_what(verb);
        } else {
            self.t_read(verb, objs[0]);
        }
    }

    fn t_read(&mut self, verb: &W, obj: usize) {
        if self.is_dark() {
            let name = data().objects[obj]
                .names
                .first()
                .cloned()
                .unwrap_or_default();
            return self.i_see_no(&name);
        }
        let hint2_used = self.hint_idx(2).map(|i| self.hint_used[i]).unwrap_or(false);
        if obj == OYSTER && !hint2_used && self.obj_toting[OYSTER] {
            return self.yesno(192, Pending::ReadOyster, false);
        } else if obj == OYSTER && hint2_used {
            self.wm(194);
        } else if obj == MESSAGE {
            self.wm(191);
        } else if obj == TABLET {
            self.wm(196);
        } else if obj == MAGAZINE {
            self.wm(190);
        } else {
            let m = self.default_msg(verb);
            self.w(m);
        }
        self.finish_turn(None);
    }

    fn t_break(&mut self, verb: &W, obj: usize) {
        if obj == VASE && self.obj_prop[VASE] == 0 {
            self.wm(198);
            if self.obj_toting[VASE] {
                let loc = self.loc;
                self.drop_at(VASE, loc);
            }
            self.obj_prop[VASE] = 2;
            self.obj_fixed[VASE] = 1;
        } else if obj == MIRROR && self.is_closed {
            self.wm(197);
            return self.wake_repository_dwarves();
        } else if obj == MIRROR {
            self.wm(148);
        } else {
            let m = self.default_msg(verb);
            self.w(m);
        }
        self.finish_turn(None);
    }

    fn t_wake(&mut self, verb: &W, obj: usize) {
        if obj == DWARF && self.is_closed {
            self.wm(199);
            self.wake_repository_dwarves();
        } else {
            let m = self.default_msg(verb);
            self.w(m);
            self.finish_turn(None);
        }
    }

    fn i_suspend(&mut self, _verb: &W) {
        self.w("Use the menu to save your game.".to_string());
        self.finish_turn(None);
    }

    fn t_suspend(&mut self) {
        // The original's `save <file>` consumes a turn but does not advance the
        // RNG (no finish_turn). Our real persistence is the autosave/menu, so
        // this is effectively a no-op acknowledgement.
        self.w("Game saved.".to_string());
    }

    fn i_hours(&mut self, _verb: &W) {
        self.w("Open all day.".to_string());
        self.finish_turn(None);
    }

    // ----- cave closing & scoring -----------------------------------------

    fn start_closing_cave(&mut self) {
        self.obj_prop[GRATE] = 0;
        self.obj_prop[FISSURE] = 0;
        self.dwarves.clear();
        self.hide(TROLL);
        self.obj_rooms[TROLL2] = data().objects[TROLL].start_rooms.clone();
        if self.obj_prop[BEAR] != 3 {
            self.hide(BEAR);
        }
        for o in [CHAIN, AXE] {
            self.obj_prop[o] = 0;
            self.obj_fixed[o] = 0;
        }
        self.wm(129);
        self.clock1 = -1;
        self.is_closing = true;
    }

    fn close_cave(&mut self) {
        let ne = 115u16;
        let sw = 116u16;
        for o in [BOTTLE, PLANT, OYSTER, LAMP, ROD, DWARF] {
            self.obj_prop[o] = if o == BOTTLE { -2 } else { -1 };
            self.drop_at(o, ne);
        }
        self.loc = ne;
        self.oldloc = ne;
        self.oldloc2 = ne;
        for o in [GRATE, SNAKE, BIRD, CAGE, ROD2, PILLOW] {
            self.obj_prop[o] = if o == BIRD || o == SNAKE { -2 } else { -1 };
            self.drop_at(o, sw);
        }
        self.obj_rooms[MIRROR] = vec![ne, sw];
        self.obj_fixed[MIRROR] = 1;
        self.is_closed = true;
        for o in self.inventory() {
            self.obj_toting[o] = false;
        }
        self.wm(132);
        self.move_to(None);
    }

    fn wake_repository_dwarves(&mut self) {
        self.wm(136);
        self.score_and_exit();
    }

    fn compute_score(&self, for_score_command: bool) -> (i32, i32) {
        let mut score = 2;
        let mut maxscore = 2;
        for t in self.treasures() {
            let value = if t > CHEST {
                16
            } else if t == CHEST {
                14
            } else {
                12
            };
            maxscore += value;
            if self.obj_prop[t] >= 0 {
                score += 2;
            }
            if self.obj_rooms[t].first() == Some(&ROOM_BUILDING) && self.obj_prop[t] == 0 {
                score += value - 2;
            }
        }
        maxscore += self.max_deaths * 10;
        score += (self.max_deaths - self.deaths) * 10;

        maxscore += 4;
        if !for_score_command && !self.gave_up {
            score += 4;
        }

        maxscore += 25;
        if self.dwarf_stage != 0 {
            score += 25;
        }

        maxscore += 25;
        if self.is_closing {
            score += 25;
        }

        maxscore += 45;
        if self.is_closed {
            score += match self.bonus {
                135 => 25,
                134 => 30,
                133 => 45,
                _ => 10,
            };
        }

        maxscore += 1;
        if self.obj_rooms[MAGAZINE].contains(&108) {
            score += 1;
        }

        for i in 0..data().hints.len() {
            if self.hint_used[i] {
                score -= data().hints[i].penalty;
            }
        }
        (score, maxscore)
    }

    fn score_and_exit(&mut self) {
        let (score, maxscore) = self.compute_score(false);
        self.w(format!(
            "\nYou scored {} out of a possible {} using {} turns.",
            score, maxscore, self.turns
        ));
        let classes = &data().class_messages;
        if classes.is_empty() {
            // No ranking table to draw on — end the run without a rank line
            // rather than underflow `classes.len() - 1`.
            self.is_done = true;
            self.mode = Mode::GameOver;
            return;
        }
        let mut idx = classes.len() - 1;
        for (i, (minimum, _)) in classes.iter().enumerate() {
            if *minimum >= score {
                idx = i;
                break;
            }
        }
        let text = classes[idx].1.clone();
        self.w(format!("\n{}\n", text));
        if idx < classes.len() - 1 {
            let d = classes[idx + 1].0 + 1 - score;
            self.w(format!(
                "To achieve the next higher rating, you need {} more point{}\n",
                d,
                if d > 1 { "s" } else { "" }
            ));
        } else {
            self.w("To achieve the next higher rating would be a neat trick!\n\nCongratulations!!\n".to_string());
        }
        self.is_done = true;
        self.mode = Mode::GameOver;
    }

    // ----- queries for the UI ---------------------------------------------

    /// The final score (and max) as shown on the game-over screen.
    pub fn final_score(&self) -> (i32, i32) {
        self.compute_score(false)
    }

    pub fn is_cave_closed(&self) -> bool {
        self.is_closed
    }

    pub fn room_name(&self) -> &'static str {
        let d = data();
        let r = &d.rooms[self.loc as usize];
        let src = if !r.short.is_empty() { &r.short } else { &r.long };
        src.lines().next().unwrap_or("")
    }

    /// Whether this (just-deserialized) save still matches the embedded
    /// `advent.dat`: every data-derived vector has the length the current data
    /// implies. A mismatch means the data changed under an old save without a
    /// `SAVE_VERSION` bump — we discard it rather than index out of bounds later.
    pub fn is_data_compatible(&self) -> bool {
        let d = data();
        let nobj = d.objects.len();
        self.obj_prop.len() == nobj
            && self.obj_rooms.len() == nobj
            && self.obj_toting.len() == nobj
            && self.obj_fixed.len() == nobj
            && self.times_described.len() == d.rooms.len()
            && self.hint_counter.len() == d.hints.len()
            && self.hint_used.len() == d.hints.len()
    }

    pub fn inventory_count(&self) -> usize {
        self.inventory().len()
    }

    pub fn lamp_is_on(&self) -> bool {
        self.obj_prop[LAMP] == 1
    }

    pub fn turn_count(&self) -> i32 {
        self.turns
    }

    #[cfg(test)]
    pub fn take_rng_ops(&mut self) -> Vec<String> {
        std::mem::take(&mut self.rng.ops)
    }
}

/// A parsed input word: its vocabulary number and the text the player typed.
#[derive(Clone, Debug)]
struct W {
    n: i32,
    text: String,
}

fn is_aboveground(n: u16) -> bool {
    (1..=8).contains(&n)
}

fn is_before_hall_of_mists(n: u16) -> bool {
    n < 15
}

fn is_after_hall_of_mists(n: u16) -> bool {
    n >= 15
}
