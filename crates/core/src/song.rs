//! The lullaby: the Defendant plays the Idiot God to sleep on a trumpet.
//!
//! After Ocarina of Time and Majora's Mask. The god's mindless dancers
//! pipe a verse (listen), and the same verse comes again for you to play
//! back on five keys, in time (play). Each note is judged by how near its
//! tick the key went down: great, good, or missed. A verse with enough of
//! its notes on time puts the god a verse deeper asleep and the next verse
//! begins; too few, and the god stirs and the verse is piped again. After
//! the last verse the whole song plays over the god while time stops round
//! it (the finale), and it sleeps.
//!
//! Throughout, the song keeps a beat, and the god fires on it: which of
//! its patterns, and on which beats, is the lullaby's data. The song says
//! what happened; the world fires and sounds it.

use crate::content::Lullaby;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// the trumpet coming out
    Intro,
    /// the dancers pipe the verse
    Listen,
    /// you play it back
    Play,
    /// the whole song, over the stopped god
    Finale,
    /// done: it sleeps
    Asleep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judge {
    Pending,
    Great,
    Good,
    Miss,
}

/// What a tick of the song did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Heard {
    /// notes sounded: (key, on the trumpet rather than the dancers' pipes)
    pub notes: Vec<(usize, bool)>,
    /// a key went down with no note near it
    pub sour: bool,
    /// a note was judged
    pub judged: Vec<Judge>,
    /// the beat, and whether it is the first of a bar
    pub beat: Option<bool>,
    /// patterns the god fires on this beat
    pub fire: Vec<usize>,
    /// a verse was played well, and the god sleeps a verse deeper
    pub passed: bool,
    /// a verse was played badly: the god stirs
    pub stirred: bool,
    /// the last verse is played: time stops round the god
    pub finale: bool,
    /// it sleeps
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    pub part: Part,
    pub verse: usize,
    /// ticks into this part, the count-in first
    pub clock: i32,
    /// this verse's notes, judged
    pub judged: Vec<Judge>,
    /// verses played well
    pub asleep: usize,
    /// beats since the song began, for the god's patterns
    pub beats: i32,
}

/// Beats in a bar, for the downbeat's drum.
const BAR: i32 = 4;

impl Song {
    pub fn new() -> Song {
        Song {
            part: Part::Intro,
            verse: 0,
            clock: 0,
            judged: Vec::new(),
            asleep: 0,
            beats: 0,
        }
    }

    /// The tick, from the start of a pass, a verse's first note can fall on.
    pub fn start(l: &Lullaby) -> i32 {
        l.count_in * l.beat
    }

    /// One tick. `pressed` are the five keys that went down on it.
    pub fn step(&mut self, l: &Lullaby, pressed: [bool; 5]) -> Heard {
        let mut h = Heard::default();
        self.clock += 1;
        match self.part {
            Part::Intro => {
                if self.clock >= l.intro {
                    self.begin(Part::Listen, l);
                }
            }
            Part::Listen | Part::Play => {
                let t = self.clock;
                if t % l.beat == 0 {
                    h.beat = Some(self.beats % BAR == 0);
                    for b in &l.beats {
                        if self.verse >= b.from_verse && (self.beats - b.at).rem_euclid(b.every) == 0 {
                            h.fire.push(b.pattern);
                        }
                    }
                    self.beats += 1;
                }
                let verse = &l.verses[self.verse];
                let first = Song::start(l);
                if self.part == Part::Listen {
                    for &(at, key) in &verse.notes {
                        if first + at == t {
                            h.notes.push((key, false));
                        }
                    }
                    // you may play along while you listen; nothing is judged
                    for (key, down) in pressed.iter().enumerate() {
                        if *down {
                            h.notes.push((key, true));
                        }
                    }
                } else {
                    self.play(l, pressed, &mut h);
                }
                if t >= first + verse.length {
                    self.end_pass(l, &mut h);
                }
            }
            Part::Finale => {
                // every verse again, one after another, on the trumpet
                let mut at = 0;
                for v in &l.verses {
                    for &(n, key) in &v.notes {
                        if at + n == self.clock - 1 {
                            h.notes.push((key, true));
                        }
                    }
                    at += v.length;
                }
                if self.clock % l.beat == 0 {
                    h.beat = Some((self.clock / l.beat) % BAR == 0);
                }
                if self.clock >= self.finale_length(l) {
                    self.part = Part::Asleep;
                    self.clock = 0;
                    h.done = true;
                }
            }
            Part::Asleep => {}
        }
        h
    }

    /// The finale's length: every verse back to back, or the lullaby's
    /// own count of beats if that is longer.
    pub fn finale_length(&self, l: &Lullaby) -> i32 {
        let all: i32 = l.verses.iter().map(|v| v.length).sum();
        all.max(l.finale_beats * l.beat)
    }

    fn begin(&mut self, part: Part, l: &Lullaby) {
        self.part = part;
        self.clock = 0;
        self.judged = vec![Judge::Pending; l.verses[self.verse].notes.len()];
    }

    /// The keys that went down, each judged against the nearest note of
    /// its key still waiting; and the notes gone by unplayed, missed.
    fn play(&mut self, l: &Lullaby, pressed: [bool; 5], h: &mut Heard) {
        let t = self.clock;
        let first = Song::start(l);
        let notes = &l.verses[self.verse].notes;
        for (key, down) in pressed.iter().enumerate() {
            if !*down {
                continue;
            }
            let near = (0..notes.len())
                .filter(|&k| notes[k].1 == key && self.judged[k] == Judge::Pending)
                .map(|k| (k, (first + notes[k].0 - t).abs()))
                .filter(|&(_, off)| off <= l.good)
                .min_by_key(|&(_, off)| off);
            match near {
                Some((k, off)) => {
                    let j = if off <= l.great { Judge::Great } else { Judge::Good };
                    self.judged[k] = j;
                    h.judged.push(j);
                    h.notes.push((key, true));
                }
                None => {
                    h.sour = true;
                    h.notes.push((key, true));
                }
            }
        }
        for k in 0..notes.len() {
            if self.judged[k] == Judge::Pending && t > first + notes[k].0 + l.good {
                self.judged[k] = Judge::Miss;
                h.judged.push(Judge::Miss);
            }
        }
    }

    fn end_pass(&mut self, l: &Lullaby, h: &mut Heard) {
        if self.part == Part::Listen {
            self.begin(Part::Play, l);
            return;
        }
        let n = self.judged.len() as i32;
        let hit = self
            .judged
            .iter()
            .filter(|j| matches!(j, Judge::Great | Judge::Good))
            .count() as i32;
        if hit * 100 >= l.pass_pct * n {
            h.passed = true;
            self.asleep += 1;
            if self.asleep >= l.verses.len() {
                h.finale = true;
                self.part = Part::Finale;
                self.clock = 0;
                return;
            }
            self.verse += 1;
        } else {
            h.stirred = true;
        }
        self.begin(Part::Listen, l);
    }

    /// The share of the song played well, Q16 from 0 to ONE, for the bar.
    pub fn sleep(&self, l: &Lullaby) -> i32 {
        let n = l.verses.len().max(1) as i32;
        crate::fx::div(self.asleep as i32 * crate::fx::ONE, n * crate::fx::ONE)
    }
}

impl Default for Song {
    fn default() -> Self {
        Song::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::content::Lullaby;

    fn lullaby() -> Lullaby {
        let c = content();
        let god = c.enemy("azathoth").unwrap();
        c.enemies[god].lullaby.clone().unwrap()
    }

    /// Plays one pass, pressing each note's key `off` ticks from its tick
    /// (none at all when `None`); returns everything heard.
    fn pass(s: &mut Song, l: &Lullaby, off: Option<i32>) -> Vec<Heard> {
        let first = Song::start(l);
        let notes = l.verses[s.verse].notes.clone();
        let part = s.part;
        let mut out = Vec::new();
        while s.part == part {
            let t = s.clock + 1;
            let mut keys = [false; 5];
            if let Some(o) = off {
                for &(at, key) in &notes {
                    if first + at + o == t {
                        keys[key] = true;
                    }
                }
            }
            out.push(s.step(l, keys));
        }
        out
    }

    #[test]
    fn the_dancers_pipe_a_verse_then_you_play_it_back_and_the_god_sleeps_a_verse_deeper() {
        let l = lullaby();
        let mut s = Song::new();
        while s.part == Part::Intro {
            s.step(&l, [false; 5]);
        }
        assert_eq!(s.part, Part::Listen);
        let heard = pass(&mut s, &l, None);
        let piped: Vec<usize> = heard.iter().flat_map(|h| h.notes.iter().map(|n| n.0)).collect();
        let verse: Vec<usize> = l.verses[0].notes.iter().map(|n| n.1).collect();
        assert_eq!(piped, verse, "the dancers pipe the verse's notes in order");
        assert_eq!(s.part, Part::Play);
        let heard = pass(&mut s, &l, Some(0));
        assert!(heard.iter().any(|h| h.passed), "played on time, it passes");
        assert_eq!(s.asleep, 1);
        assert_eq!(s.verse, 1);
        assert_eq!(s.part, Part::Listen);
    }

    #[test]
    fn a_note_is_judged_by_how_near_its_tick_the_key_went_down() {
        let l = lullaby();
        let mut s = Song::new();
        while s.part != Part::Play {
            s.step(&l, [false; 5]);
        }
        let heard = pass(&mut s, &l, Some(l.great + 1));
        let judged: Vec<Judge> = heard.iter().flat_map(|h| h.judged.clone()).collect();
        assert!(judged.iter().all(|j| *j == Judge::Good), "{judged:?}");
        while s.part != Part::Play {
            s.step(&l, [false; 5]);
        }
        let heard = pass(&mut s, &l, Some(-l.good - 1));
        let judged: Vec<Judge> = heard.iter().flat_map(|h| h.judged.clone()).collect();
        assert!(
            judged.iter().all(|j| *j == Judge::Miss),
            "too early is a miss: {judged:?}"
        );
        assert!(heard.iter().any(|h| h.sour), "and the key sounds sour");
    }

    #[test]
    fn a_verse_played_badly_stirs_the_god_and_comes_again() {
        let l = lullaby();
        let mut s = Song::new();
        while s.part != Part::Play {
            s.step(&l, [false; 5]);
        }
        let heard = pass(&mut s, &l, None);
        assert!(heard.iter().any(|h| h.stirred));
        assert_eq!((s.verse, s.asleep, s.part), (0, 0, Part::Listen));
    }

    #[test]
    fn every_verse_played_well_ends_in_the_finale_and_sleep() {
        let l = lullaby();
        let mut s = Song::new();
        let mut finale = false;
        let mut done = false;
        for _ in 0..200_000 {
            let keys = if s.part == Part::Play {
                let first = Song::start(&l);
                let mut k = [false; 5];
                for &(at, key) in &l.verses[s.verse].notes {
                    if first + at == s.clock + 1 {
                        k[key] = true;
                    }
                }
                k
            } else {
                [false; 5]
            };
            let h = s.step(&l, keys);
            finale |= h.finale;
            done |= h.done;
            if done {
                break;
            }
        }
        assert!(finale && done, "{:?}", s.part);
        assert_eq!(s.asleep, l.verses.len());
        assert_eq!(s.sleep(&l), crate::fx::ONE);
    }

    #[test]
    fn the_god_fires_on_the_beat_and_more_as_the_verses_go_on() {
        let l = lullaby();
        let mut s = Song::new();
        while s.part == Part::Intro {
            s.step(&l, [false; 5]);
        }
        let mut beats = Vec::new();
        let mut fired = 0;
        let mut tick = 0;
        while s.part == Part::Listen {
            let h = s.step(&l, [false; 5]);
            tick += 1;
            if h.beat.is_some() {
                beats.push(tick);
            }
            fired += h.fire.len();
        }
        assert!(beats.windows(2).all(|w| w[1] - w[0] == l.beat), "{beats:?}");
        assert!(fired > 0);
        // a later verse's beats fire more
        let later = l.beats.iter().filter(|b| b.from_verse > 0).count();
        assert!(later > 0, "patterns join as the verses go on");
    }
}
