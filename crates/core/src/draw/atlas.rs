//! The map's dioramas: each zone as a small floating island that holds the
//! zone at a glance, turning slowly over the map. Each stands on a round
//! base of its own ground over a cone of its own rock. A zone beaten wears
//! a gold ring and a star over it; a locked one is drawn greyed and dim; a
//! sealed one is a dark shape only. The pointer over one lifts and
//! brightens it.

use super::rod;
use crate::content::{Content, Rgb};
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{boxed, mix, shade, Batch, PASS_SOLID};

/// How a zone stands on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Open,
    Locked,
    Beaten,
    Sealed,
}

/// A point or a size in a diorama, in hundredths of its width.
type P = [i32; 3];

fn unit(p: P) -> V3 {
    v3(p[0] * ONE / 100, p[1] * ONE / 100, p[2] * ONE / 100)
}

/// A diorama's frame and how it is painted.
struct D<'a> {
    b: &'a mut Batch,
    f: Affine,
    /// its width, in the world
    k: i32,
    standing: Standing,
    hover: i32,
    ink: Rgb,
}

impl D<'_> {
    /// A colour as this diorama's standing and the pointer paint it.
    fn paint(&self, rgb: Rgb) -> Rgb {
        let grey = |c: Rgb| {
            let l = (c[0] * 30 + c[1] * 59 + c[2] * 11) / 100;
            [l, l, l]
        };
        let c = match self.standing {
            Standing::Open | Standing::Beaten => rgb,
            Standing::Locked => shade(mix(rgb, grey(rgb), ONE * 3 / 4), 62),
            Standing::Sealed => mix(shade(grey(rgb), 18), self.ink, ONE / 2),
        };
        mix(c, [ONE; 3], self.hover / 6)
    }

    fn lit(&self, g: i32) -> i32 {
        let g = match self.standing {
            Standing::Open | Standing::Beaten => g,
            Standing::Locked => g / 4,
            Standing::Sealed => 0,
        };
        (g + self.hover / 5).min(ONE)
    }

    fn at(&self, p: P) -> V3 {
        self.f.apply(unit(p))
    }

    fn width(&self, x: i32) -> i32 {
        fx::mul(x * ONE / 100, self.k)
    }

    /// The same frame moved to `p`, turned by `yaw` and grown to `pct` in
    /// a hundred, for a thing with parts of its own (a ship, a house).
    fn sub(&mut self, p: P, yaw: i32, pct: i32) -> D<'_> {
        let k = ONE * pct / 100;
        D {
            f: self
                .f
                .then(&Affine::translate(unit(p)))
                .then(&Affine::rot_y(yaw))
                .then(&Affine::scale(v3(k, k, k))),
            k: fx::mul(self.k, k),
            b: &mut *self.b,
            ..*self
        }
    }

    fn shape(&mut self, mesh: Mesh, a: &Affine, rgb: Rgb, glow: i32, ink: bool) {
        let (rgb, glow) = (self.paint(rgb), self.lit(glow));
        let a = self.f.then(a);
        if ink {
            self.b.inked(mesh, &a, rgb, glow);
        } else {
            self.b.push(mesh, &a, rgb, glow, PASS_SOLID);
        }
    }

    /// An inked shape at `p`, `s` across.
    fn put(&mut self, mesh: Mesh, p: P, s: P, rgb: Rgb) {
        self.shape(mesh, &boxed(unit(p), unit(s)), rgb, 0, true);
    }

    /// An inked shape that gives light.
    fn lit_put(&mut self, mesh: Mesh, p: P, s: P, rgb: Rgb, glow: i32) {
        self.shape(mesh, &boxed(unit(p), unit(s)), rgb, glow, true);
    }

    /// A shape with no outline: windows, water, seams.
    fn flat(&mut self, mesh: Mesh, p: P, s: P, rgb: Rgb, glow: i32) {
        self.shape(mesh, &boxed(unit(p), unit(s)), rgb, glow, false);
    }

    /// A shape at `p` turned by `r`, `s` across.
    #[allow(clippy::too_many_arguments)]
    fn turned(&mut self, mesh: Mesh, p: P, r: Affine, s: P, rgb: Rgb, glow: i32, ink: bool) {
        let a = Affine::translate(unit(p)).then(&r).then(&Affine::scale(unit(s)));
        self.shape(mesh, &a, rgb, glow, ink);
    }

    /// A rod from `a` to `c`, `w` across.
    fn rod(&mut self, a: P, c: P, w: i32, rgb: Rgb, glow: i32, ink: bool) {
        let (pa, pc, w) = (self.at(a), self.at(c), self.width(w));
        let (rgb, glow) = (self.paint(rgb), self.lit(glow));
        rod(self.b, pa, pc, w, rgb, glow, ink);
    }

    /// A soft halo round `p`, `r` across.
    fn halo(&mut self, p: P, r: i32, rgb: Rgb, strength: i32) {
        if self.standing == Standing::Sealed {
            return;
        }
        let s = if self.standing == Standing::Locked {
            strength / 3
        } else {
            strength
        };
        let w = self.width(r);
        let at = self.at(p);
        let rgb = self.paint(rgb);
        self.b.glow(Mesh::Sphere, &boxed(at, v3(w, w, w)), rgb, s);
    }

    /// The island: a disc of `top` over a cone of `rock`.
    fn base(&mut self, top: Rgb, rock: Rgb) {
        self.put(Mesh::Cyl, [0, -4, 0], [100, 8, 100], top);
        self.turned(
            Mesh::Cone,
            [0, -26, 0],
            Affine::rot_x(fx::HALF),
            [92, 36, 92],
            rock,
            0,
            true,
        );
    }
}

/// A point `r` hundredths out from the middle at `angle`, `y` up.
fn round(angle: i32, r: i32, y: i32) -> P {
    [
        fx::mul(fx::cos(angle), r * ONE) / ONE,
        y,
        fx::mul(fx::sin(angle), r * ONE) / ONE,
    ]
}

/// A zone's diorama, centred at `at`, `size` metres across (Q16), turned by
/// `spin` (fx::TURN units), lit up by `hover` (0..ONE, when the pointer is
/// over it).
#[allow(clippy::too_many_arguments)]
pub fn diorama(
    b: &mut Batch,
    c: &Content,
    zone: &str,
    at: V3,
    size: i32,
    spin: i32,
    hover: i32,
    standing: Standing,
) {
    let hover = hover.clamp(0, ONE);
    let lift = fx::mul(hover, size / 10);
    let centre = at.add(v3(0, lift, 0));
    let mut d = D {
        f: Affine::translate(centre)
            .then(&Affine::rot_y(spin))
            .then(&Affine::scale(v3(size, size, size))),
        b,
        k: size,
        standing,
        hover,
        ink: c.pal.c("ink"),
    };
    match zone {
        "waiting_city" => waiting_city(&mut d, c),
        "white_waste" => white_waste(&mut d, c),
        "glass_house" => glass_house(&mut d, c),
        "the_ascent" => ascent(&mut d, c),
        "idiot_god" => idiot_god(&mut d, c),
        "home" => home(&mut d, c),
        "water_gap" => water_gap(&mut d, c),
        "far_shore" => far_shore(&mut d, c),
        "vault" => vault(&mut d, c),
        _ => {
            d.base(c.pal.c("ground"), c.pal.c("crater"));
            d.put(Mesh::Blob, [-8, 4, 6], [30, 16, 24], c.pal.c("stone"));
        }
    }
    if standing == Standing::Beaten {
        let gold = c.pal.c("gold");
        d.flat(Mesh::Ring, [0, -4, 0], [110, 4, 110], gold, ONE * 2 / 3);
        // the star stays upright over it while it turns
        let star = at.add(v3(0, lift + size * 7 / 10, 0));
        let s = size / 9;
        let spun = Affine::translate(star)
            .then(&Affine::rot_y(-spin * 2))
            .then(&Affine::scale(v3(s, s * 3 / 2, s)));
        d.b.inked(Mesh::Octa, &spun, d.paint(gold), d.lit(ONE * 3 / 4));
        d.b.glow(Mesh::Sphere, &boxed(star, v3(s * 3, s * 3, s * 3)), gold, ONE / 4);
    }
    if hover > 0 {
        let w = size * 13 / 10;
        let light = d.paint(c.pal.c("light"));
        d.b.glow(Mesh::Sphere, &boxed(centre, v3(w, w, w)), light, hover / 6);
    }
}

/// The Waiting City at dusk: grey-cream towers with lit windows round a
/// plaza and its fountain, the courthouse's clock tower over them.
fn waiting_city(d: &mut D, c: &Content) {
    d.base(c.pal.c("paving_dark"), c.pal.c("ground"));
    d.flat(Mesh::Cyl, [0, 1, 0], [40, 2, 40], c.pal.c("paving"), 0);
    d.put(Mesh::Cyl, [0, 3, 0], [12, 4, 12], c.pal.c("stone"));
    d.flat(Mesh::Cyl, [0, 5, 0], [9, 1, 9], c.pal.c("water"), ONE / 3);
    let window = c.pal.c("window");
    let towers = [
        (deg(10), 34, 26, 14, "atlas_tower"),
        (deg(55), 34, 44, 12, "atlas_tower_2"),
        (deg(100), 33, 22, 16, "atlas_tower"),
        (deg(145), 34, 36, 13, "atlas_tower_2"),
        (deg(190), 34, 28, 15, "atlas_tower"),
        (deg(235), 33, 50, 12, "atlas_tower"),
        (deg(280), 35, 24, 14, "atlas_tower_2"),
        (deg(325), 34, 38, 13, "atlas_tower"),
    ];
    for (a, r, h, w, name) in towers {
        let p = round(a, r, 0);
        d.put(Mesh::Cube, [p[0], h / 2, p[2]], [w, h, w], c.pal.c(name));
        // two bands of lit windows on each of the faces turned outward
        let sx = if p[0] < 0 { -1 } else { 1 };
        let sz = if p[2] < 0 { -1 } else { 1 };
        for y in [h * 2 / 5, h * 3 / 4] {
            d.flat(
                Mesh::Cube,
                [p[0] + sx * (w / 2 + 1), y, p[2]],
                [1, 3, w * 3 / 4],
                window,
                ONE * 2 / 3,
            );
            d.flat(
                Mesh::Cube,
                [p[0], y, p[2] + sz * (w / 2 + 1)],
                [w * 3 / 4, 3, 1],
                window,
                ONE * 2 / 3,
            );
        }
    }
    // the clock tower, behind the fountain
    d.put(Mesh::Cube, [0, 22, -14], [8, 44, 8], c.pal.c("court"));
    d.put(Mesh::Cone, [0, 50, -14], [10, 12, 10], c.pal.c("roof"));
    for (p, s) in [([0, 38, -9], [5, 5, 1]), ([5, 38, -14], [1, 5, 5])] {
        d.flat(Mesh::Cube, p, s, c.pal.c("clock"), ONE / 2);
    }
    d.halo([0, 6, 0], 30, c.pal.c("lamp"), ONE / 8);
}

/// The White Waste: an ice shelf split by a crevasse, spikes of ice, and
/// the expedition's plane broken on it, its lamp still lit.
fn white_waste(d: &mut D, c: &Content) {
    d.base(c.pal.c("ice_snow"), c.pal.c("ice_rock"));
    d.flat(Mesh::Cube, [0, 1, 0], [7, 2, 92], c.pal.c("ice_water"), 0);
    d.flat(Mesh::Cube, [0, 2, 0], [3, 2, 92], c.pal.c("ice_window_dark"), 0);
    d.put(Mesh::Cube, [-24, 4, -6], [36, 8, 52], c.pal.c("ice_white"));
    d.put(Mesh::Cube, [-20, 10, 4], [22, 6, 26], c.pal.c("ice_snow_dark"));
    d.put(Mesh::Cube, [22, 3, -14], [30, 6, 36], c.pal.c("ice_white"));
    let spikes = [
        ([-30, 0, -24], 42, 11, deg(8), "ice_blue"),
        ([-38, 0, 8], 30, 9, -deg(12), "ice_deep"),
        ([-16, 0, -34], 26, 8, deg(16), "ice_glow"),
        ([-24, 8, 0], 22, 7, -deg(6), "ice_white"),
        ([30, 0, -28], 38, 10, -deg(10), "ice_blue"),
        ([38, 0, 0], 24, 8, deg(14), "ice_deep"),
        ([14, 0, -38], 20, 7, 0, "ice_glow"),
    ];
    for (p, h, w, tilt, name) in spikes {
        let g = if name == "ice_glow" { ONE / 3 } else { 0 };
        d.turned(
            Mesh::Cone,
            [p[0], p[1] + h / 2, p[2]],
            Affine::rot_z(tilt),
            [w, h, w],
            c.pal.c(name),
            g,
            true,
        );
    }
    // the plane, nose down in the snow, a wing torn off beside it
    let metal = c.pal.c("ice_metal");
    let red = c.pal.c("ice_red");
    let mut plane = d.sub([16, 7, 20], deg(30), 100);
    plane.turned(
        Mesh::Cyl,
        [0, 0, 0],
        Affine::rot_z(fx::QUARTER - deg(14)),
        [11, 46, 11],
        metal,
        0,
        true,
    );
    plane.put(Mesh::Cube, [0, 1, 0], [12, 2, 22], red);
    plane.put(Mesh::Wedge, [20, 11, 0], [8, 12, 2], red);
    plane.put(Mesh::Cube, [20, 7, 0], [6, 2, 16], red);
    plane.flat(Mesh::Cube, [-14, 2, 0], [5, 4, 9], c.pal.c("ice_window_dark"), 0);
    d.turned(
        Mesh::Cube,
        [36, 7, 22],
        Affine::rot_x(deg(30)).then(&Affine::rot_z(deg(20))),
        [12, 2, 22],
        red,
        0,
        true,
    );
    d.lit_put(Mesh::Sphere, [6, 4, 30], [4, 4, 4], c.pal.c("ice_lamp"), ONE);
    d.halo([6, 4, 30], 18, c.pal.c("ice_lamp"), ONE / 4);
}

/// The Glass House on Calder Hill: a modernist house of glass and white
/// slabs on a dark lawn, lit within, hedges and a pool.
fn glass_house(d: &mut D, c: &Content) {
    let lawn = c.pal.c("grass_night");
    d.base(lawn, c.pal.c("house_marble_dark"));
    d.put(Mesh::Sphere, [0, 0, 0], [92, 34, 92], lawn);
    let marble = c.pal.c("house_marble");
    let glass = c.pal.c("glass");
    let frame = c.pal.c("house_ink");
    // the house, a third bigger than the lawn's things, on the hilltop
    let mut h = d.sub([0, 16, 0], -deg(15), 130);
    // the ground floor: a glass box between a floor and a roof slab
    h.put(Mesh::Cube, [0, 1, 0], [42, 2, 28], marble);
    h.lit_put(Mesh::Cube, [0, 7, 0], [36, 10, 22], glass, ONE / 4);
    h.put(Mesh::Cube, [3, 13, 0], [48, 2, 30], marble);
    // the upper floor, set back and lit warm
    h.lit_put(
        Mesh::Cube,
        [8, 19, -2],
        [24, 10, 18],
        c.pal.c("house_candle"),
        ONE / 2,
    );
    h.put(Mesh::Cube, [8, 25, -2], [30, 2, 22], marble);
    // a stone wall through both, and the panes' frames
    h.put(
        Mesh::Cube,
        [-15, 12, 0],
        [4, 26, 26],
        c.pal.c("house_marble_dark"),
    );
    for x in [-6, 3, 12] {
        h.flat(Mesh::Cube, [x, 7, -11], [1, 10, 1], frame, 0);
        h.flat(Mesh::Cube, [x, 7, 11], [1, 10, 1], frame, 0);
    }
    for x in [2, 14] {
        h.flat(Mesh::Cube, [x, 19, -11], [1, 10, 1], frame, 0);
    }
    // the pool, the hedges, two dark trees
    d.flat(
        Mesh::Cube,
        [24, 11, 20],
        [18, 2, 10],
        c.pal.c("house_moon"),
        ONE / 2,
    );
    let hedge = c.pal.c("hedge");
    d.put(Mesh::Cube, [-24, 9, 22], [22, 6, 6], hedge);
    d.put(Mesh::Cube, [-30, 8, -18], [6, 6, 22], hedge);
    d.put(Mesh::Cone, [30, 18, -20], [12, 26, 12], hedge);
    d.put(Mesh::Cone, [-34, 14, 2], [10, 20, 10], hedge);
    d.halo([6, 30, 0], 50, c.pal.c("house_candle"), ONE / 6);
}

/// The Ascent: a spire of warm stone, its switchback path, a chapel on a
/// ledge near the snow at the top, cloud round its foot.
fn ascent(d: &mut D, c: &Content) {
    d.base(c.pal.c("ascent_moss"), c.pal.c("ascent_rock"));
    let stone = c.pal.c("ascent_stone");
    // the spire is a cone 60 across and 90 high
    d.put(Mesh::Cone, [0, 45, 0], [60, 90, 60], stone);
    d.put(
        Mesh::Cone,
        [16, 20, 10],
        [40, 40, 40],
        c.pal.c("ascent_stone_dark"),
    );
    d.put(Mesh::Cone, [-18, 14, -6], [34, 28, 34], c.pal.c("ascent_rock"));
    d.put(Mesh::Cone, [0, 78, 0], [17, 24, 17], c.pal.c("ascent_snow"));
    // the path, winding round the spire up to the chapel
    let r = |y: i32| 30 * (90 - y) / 90 + 2;
    let path = c.pal.c("ascent_door");
    let mut last = None;
    for i in 0..10 {
        let y = 4 + 6 * i;
        let a = fx::QUARTER + deg(36) * (i - 9);
        let p = round(a, r(y), y);
        if let Some(q) = last {
            d.rod(q, p, 4, path, ONE / 4, true);
        }
        last = Some(p);
    }
    // the chapel on its ledge
    let rr = r(62);
    d.put(Mesh::Cube, [0, 61, rr + 3], [14, 3, 12], stone);
    d.put(Mesh::Cube, [0, 66, rr + 4], [7, 7, 8], c.pal.c("ascent_light"));
    let roof = c.pal.c("roof");
    d.put(Mesh::Wedge, [0, 72, rr + 2], [9, 5, 4], roof);
    d.turned(
        Mesh::Wedge,
        [0, 72, rr + 6],
        Affine::rot_y(fx::HALF),
        [9, 5, 4],
        roof,
        0,
        true,
    );
    d.put(Mesh::Cone, [0, 77, rr + 4], [3, 6, 3], roof);
    d.flat(
        Mesh::Cube,
        [0, 65, rr + 8],
        [3, 4, 1],
        c.pal.c("ascent_door"),
        ONE,
    );
    d.halo([0, 66, rr + 8], 16, c.pal.c("ascent_sun"), ONE / 5);
    // cloud below
    let cloud = c.pal.c("ascent_cloud");
    for (a, s) in [(deg(30), 22), (deg(160), 26), (deg(260), 20)] {
        d.put(Mesh::Blob, round(a, 38, 3), [s, s / 2, s * 3 / 4], cloud);
    }
}

/// Outside, the Idiot God: a dark orb with eyes, ringed, among floating
/// shards of crystal, over a broken disc veined with gold.
fn idiot_god(d: &mut D, c: &Content) {
    d.base(c.pal.c("god_stone_dark"), c.pal.c("god_flesh_dark"));
    let seam = c.pal.c("god_seam");
    for a in [deg(20), deg(140), deg(250)] {
        let p = round(a, 22, 1);
        d.turned(Mesh::Cube, p, Affine::rot_y(-a), [36, 1, 2], seam, ONE / 2, false);
    }
    let y = 42;
    d.put(Mesh::Sphere, [0, y, 0], [44, 44, 44], c.pal.c("eldritch_flesh"));
    // its eyes, more than one so one is always looking
    for (a, s) in [
        (fx::QUARTER, 20),
        (fx::QUARTER + deg(130), 12),
        (fx::QUARTER - deg(120), 9),
    ] {
        let out = |r: i32, dy: i32| round(a, r, y + dy);
        let dy = if s < 20 { 8 } else { 0 };
        d.turned(
            Mesh::Sphere,
            out(19, dy),
            Affine::rot_y(-a),
            [s / 2, s, s],
            c.pal.c("god_eye"),
            ONE / 3,
            true,
        );
        d.turned(
            Mesh::Sphere,
            out(22, dy),
            Affine::rot_y(-a),
            [s / 4, s * 3 / 5, s * 3 / 5],
            c.pal.c("god_iris"),
            ONE * 2 / 3,
            false,
        );
        d.turned(
            Mesh::Cube,
            out(23, dy),
            Affine::rot_y(-a),
            [2, s * 2 / 5, s / 8 + 1],
            c.pal.c("eldritch_void"),
            0,
            false,
        );
    }
    // its rings, tilted against each other
    d.turned(
        Mesh::Ring,
        [0, y, 0],
        Affine::rot_z(deg(22)).then(&Affine::rot_x(deg(10))),
        [78, 4, 78],
        c.pal.c("violet_deep"),
        ONE / 2,
        false,
    );
    d.turned(
        Mesh::Ring,
        [0, y, 0],
        Affine::rot_z(-deg(30)).then(&Affine::rot_x(-deg(18))),
        [66, 2, 66],
        seam,
        ONE / 2,
        false,
    );
    // shards of crystal and stone, adrift
    let shards = [
        (deg(0), 40, 18, "god_crystal"),
        (deg(60), 34, 66, "violet"),
        (deg(120), 42, 30, "god_stone"),
        (deg(185), 38, 60, "god_crystal"),
        (deg(240), 44, 22, "violet"),
        (deg(300), 36, 72, "god_crystal"),
    ];
    for (a, r, h, name) in shards {
        let g = if name == "god_stone" { 0 } else { ONE / 2 };
        d.turned(
            Mesh::Octa,
            round(a, r, h),
            Affine::rot_z(a / 3),
            [6, 14, 6],
            c.pal.c(name),
            g,
            true,
        );
    }
    d.halo([0, y, 0], 80, c.pal.c("violet"), ONE / 4);
}

/// Home on a Saturday morning: a small house, a window lit, a tree, and
/// Chumbles sitting on the roof's ridge.
fn home(d: &mut D, c: &Content) {
    d.base(c.pal.c("moss"), c.pal.c("home_floor_dark"));
    d.flat(Mesh::Cube, [-6, 1, -30], [8, 1, 30], c.pal.c("stone"), 0);
    d.put(Mesh::Cube, [0, 12, 0], [40, 24, 30], c.pal.c("home_wall"));
    let roof = c.pal.c("roof");
    d.put(Mesh::Wedge, [0, 31, -8], [46, 14, 17], roof);
    d.turned(
        Mesh::Wedge,
        [0, 31, 8],
        Affine::rot_y(fx::HALF),
        [46, 14, 17],
        roof,
        0,
        true,
    );
    d.put(Mesh::Cube, [13, 36, 6], [5, 10, 5], c.pal.c("trim"));
    d.flat(Mesh::Cube, [-6, 7, -15], [7, 14, 1], c.pal.c("home_wood"), 0);
    let window = c.pal.c("home_window");
    d.flat(Mesh::Cube, [9, 14, -15], [10, 8, 1], window, ONE * 3 / 4);
    d.flat(Mesh::Cube, [-8, 14, 15], [10, 8, 1], window, ONE / 2);
    d.flat(Mesh::Cube, [20, 14, 0], [1, 8, 10], window, ONE / 2);
    d.halo([9, 14, -18], 22, c.pal.c("home_light"), ONE / 5);
    // Chumbles on the ridge
    let cream = c.pal.c("cat_cream");
    let point = c.pal.c("cat_point_dark");
    d.put(Mesh::Sphere, [-6, 41, 0], [6, 7, 6], cream);
    d.put(Mesh::Sphere, [-6, 46, -1], [5, 5, 5], cream);
    d.flat(Mesh::Sphere, [-6, 46, -3], [3, 3, 1], c.pal.c("cat_point"), 0);
    d.put(Mesh::Cone, [-8, 49, -1], [2, 3, 2], point);
    d.put(Mesh::Cone, [-4, 49, -1], [2, 3, 2], point);
    d.rod([-6, 39, 3], [-10, 43, 6], 2, point, 0, true);
    // a tree
    d.put(Mesh::Cyl, [-28, 8, 14], [4, 16, 4], c.pal.c("trunk"));
    d.put(Mesh::Sphere, [-28, 22, 14], [18, 16, 18], c.pal.c("home_plant"));
}

/// The Water Gap: a canyon of red sandstone, the river through it out to
/// a lake, and on the lake's shore the Gallows Oak, dark and huge, its
/// gold eyes open.
fn water_gap(d: &mut D, c: &Content) {
    d.base(c.pal.c("sandstone_pale"), c.pal.c("sandstone_dark"));
    let water = c.pal.c("river_water");
    d.flat(Mesh::Cube, [-20, 1, 0], [56, 2, 14], water, ONE / 6);
    d.flat(Mesh::Cyl, [24, 1, 4], [36, 2, 36], water, ONE / 6);
    d.flat(Mesh::Cyl, [24, 2, 4], [20, 1, 20], c.pal.c("river_deep"), 0);
    let foam = c.pal.c("river_foam");
    for x in [-36, -22, -8] {
        d.flat(Mesh::Cube, [x, 2, (x / 4) % 4], [6, 1, 2], foam, ONE / 4);
    }
    // the canyon's two walls, banded
    for s in [-1, 1] {
        // a slope down to the water, a band of rust, a mesa on top
        let face = if s < 0 {
            Affine::rot_y(fx::HALF)
        } else {
            Affine::IDENTITY
        };
        let (dark, mid) = (c.pal.c("sandstone_dark"), c.pal.c("sandstone"));
        d.turned(Mesh::Wedge, [-16, 6, s * 18], face, [46, 12, 20], mid, 0, true);
        d.put(Mesh::Cube, [-16, 9, s * 33], [32, 18, 10], dark);
        d.put(
            Mesh::Cube,
            [-22, 20, s * 31],
            [24, 4, 12],
            c.pal.c("sandstone_pale"),
        );
    }
    // the Gallows Oak
    let bark = c.pal.c("oak_bark");
    let leaf = c.pal.c("oak_leaf");
    d.put(Mesh::Cyl, [30, 16, -26], [9, 32, 9], bark);
    d.rod([30, 26, -26], [18, 40, -22], 4, bark, 0, true);
    d.rod([30, 28, -26], [40, 42, -30], 4, bark, 0, true);
    d.put(Mesh::Sphere, [30, 46, -26], [34, 18, 30], leaf);
    d.put(Mesh::Sphere, [16, 42, -20], [20, 12, 18], leaf);
    d.put(Mesh::Sphere, [42, 42, -32], [18, 12, 16], leaf);
    d.rod([18, 39, -22], [18, 28, -22], 1, c.pal.c("ascent_rope"), 0, false);
    let eye = c.pal.c("oak_eye");
    for p in [[30, 22, -21], [24, 44, -12], [38, 46, -17]] {
        d.lit_put(Mesh::Sphere, p, [3, 2, 3], eye, ONE);
    }
    d.halo([30, 22, -21], 14, eye, ONE / 5);
}

/// The Far Shore: a disc of ocean round a whirlpool, small ships riding it
/// round, and a storm over one side with its lightning.
fn far_shore(d: &mut D, c: &Content) {
    d.base(c.pal.c("ocean"), c.pal.c("ocean_deep"));
    // the whirlpool: rings of water and foam, each turned a little further
    // in, down to a dark eye
    let rings = [
        (60, "rogue_wave", 0),
        (48, "ocean_deep", 2),
        (38, "wash", 3),
        (28, "ocean_deep", 4),
        (20, "river_foam", 4),
        (12, "deep_sky", 3),
    ];
    for (i, (w, name, off)) in rings.into_iter().enumerate() {
        let a = deg(70) * i as i32;
        let p = round(a, off, 1 + i as i32 / 2);
        let g = if name == "river_foam" { ONE / 4 } else { 0 };
        d.flat(Mesh::Cyl, p, [w, 1, w], c.pal.c(name), g);
    }
    // the ships, sailing round it
    for (k, a) in [deg(20), deg(140), deg(260)].into_iter().enumerate() {
        let mut s = d.sub(round(a, 37, 2), -a, 100);
        let sail = if k == 1 { "pirate_red" } else { "sail" };
        s.put(Mesh::Cube, [0, 1, 0], [4, 3, 11], c.pal.c("hull"));
        s.put(Mesh::Wedge, [0, 1, -8], [4, 3, 5], c.pal.c("hull_dark"));
        s.rod([0, 2, 0], [0, 14, 0], 1, c.pal.c("hull_dark"), 0, false);
        s.put(Mesh::Cube, [0, 9, 0], [8, 7, 1], c.pal.c(sail));
        s.flat(Mesh::Cube, [0, 15, 1], [1, 2, 3], c.pal.c("pirate_navy"), 0);
    }
    // the storm
    let cloud = c.pal.c("storm_cloud");
    d.put(Mesh::Blob, [-24, 44, -14], [28, 12, 22], cloud);
    d.put(Mesh::Blob, [-12, 48, -22], [22, 12, 18], c.pal.c("storm_sky"));
    d.put(Mesh::Blob, [-32, 46, -2], [18, 10, 16], cloud);
    for x in [-34, -26, -18, -10] {
        d.rod(
            [x, 38, -14 + x / 4],
            [x + 2, 6, -14 + x / 4],
            1,
            c.pal.c("storm_fog"),
            0,
            false,
        );
    }
    let bolt = c.pal.c("lightning");
    d.rod([-20, 40, -18], [-26, 22, -12], 2, bolt, ONE, false);
    d.rod([-26, 22, -12], [-18, 4, -10], 2, bolt, ONE, false);
    d.halo([-22, 24, -14], 30, bolt, ONE / 6);
}

/// The Vault under Stillwater: white and chrome steps, a ring of cyan light
/// standing on edge between chrome pillars, slabs of black glass.
fn vault(d: &mut D, c: &Content) {
    let white = c.pal.c("vault_white");
    let chrome = c.pal.c("chrome");
    let black = c.pal.c("black_glass");
    let cyan = c.pal.c("vault_cyan");
    d.base(black, chrome);
    d.put(Mesh::Cyl, [0, 2, 0], [74, 4, 74], white);
    d.put(Mesh::Cyl, [0, 6, 0], [52, 4, 52], chrome);
    d.flat(Mesh::Ring, [0, 4, 0], [76, 1, 76], cyan, ONE);
    // the ring on edge, its black glass inside it
    let edge = Affine::rot_x(fx::QUARTER);
    d.turned(Mesh::Ring, [0, 32, 0], edge, [52, 8, 52], chrome, 0, false);
    d.turned(Mesh::Ring, [0, 32, 0], edge, [46, 6, 46], cyan, ONE, false);
    d.turned(Mesh::Cyl, [0, 32, 0], edge, [44, 2, 44], black, ONE / 8, false);
    for x in [-30, 30] {
        d.put(Mesh::Cube, [x, 22, 0], [6, 36, 8], chrome);
        d.put(Mesh::Cube, [x, 41, 0], [10, 3, 10], white);
    }
    d.put(Mesh::Cube, [0, 9, 0], [36, 2, 10], white);
    // white monoliths and black glass round the edge
    for (a, h, name) in [
        (deg(40), 26, "vault_white"),
        (deg(130), 18, "black_glass"),
        (deg(220), 30, "vault_white"),
        (deg(310), 20, "black_glass"),
    ] {
        let p = round(a, 40, h / 2);
        d.turned(
            Mesh::Cube,
            p,
            Affine::rot_y(-a),
            [5, h, 10],
            c.pal.c(name),
            0,
            true,
        );
    }
    d.halo([0, 32, 0], 70, cyan, ONE / 4);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::render::DYNAMIC_BUFFER;

    const STANDINGS: [Standing; 4] = [
        Standing::Open,
        Standing::Locked,
        Standing::Beaten,
        Standing::Sealed,
    ];

    fn drawn(zone: &str, standing: Standing, hover: i32) -> Batch {
        let c = content();
        let mut b = Batch::new(c.pal.c("ink"));
        diorama(&mut b, &c, zone, v3(0, 0, 0), ONE * 4, deg(30), hover, standing);
        b
    }

    /// Each zone's instance count (open): the map's draw cost.
    const MOST: usize = 120;
    const LEAST: usize = 20;

    #[test]
    fn every_zone_draws_its_own_diorama_within_budget() {
        let c = content();
        let mut seen: Vec<Vec<i32>> = Vec::new();
        for z in &c.zones {
            for s in STANDINGS {
                for hover in [0, ONE] {
                    let n = drawn(&z.id, s, hover).count();
                    assert!(
                        (LEAST..=MOST).contains(&n),
                        "{} ({s:?}, hover {hover}) drew {n} instances",
                        z.id
                    );
                }
            }
            let (values, _) = drawn(&z.id, Standing::Open, 0).finish(DYNAMIC_BUFFER);
            assert!(!seen.contains(&values), "{} draws the same as another zone", z.id);
            seen.push(values);
        }
    }

    #[test]
    fn an_unknown_zone_is_still_a_rock() {
        let n = drawn("nowhere", Standing::Open, 0).count();
        assert!((4..LEAST).contains(&n), "an unknown zone drew {n}");
    }

    #[test]
    fn a_locked_zone_is_greyed_and_a_sealed_one_dark() {
        let colours = |s: Standing| {
            let (v, _) = drawn("water_gap", s, 0).finish(DYNAMIC_BUFFER);
            v.chunks(crate::render::INSTANCE_VALUES)
                .map(|i| [i[12], i[13], i[14]])
                .collect::<Vec<_>>()
        };
        let open = colours(Standing::Open);
        let locked = colours(Standing::Locked);
        let sealed = colours(Standing::Sealed);
        assert_eq!(open.len(), locked.len());
        let total = |cs: &[[i32; 3]]| cs.iter().map(|c| (c[0] + c[1] + c[2]) as i64).sum::<i64>();
        // spread between channels: how coloured it is
        let chroma = |cs: &[[i32; 3]]| {
            cs.iter()
                .map(|c| (c.iter().max().unwrap() - c.iter().min().unwrap()) as i64)
                .sum::<i64>()
        };
        assert!(open != locked, "locked is drawn as open");
        assert!(chroma(&locked) * 2 < chroma(&open), "locked is not greyed");
        assert!(
            total(&sealed) < total(&locked),
            "sealed is not darker than locked"
        );
    }

    #[test]
    fn a_beaten_zone_wears_gold_and_hover_lifts_it() {
        let open = drawn("vault", Standing::Open, 0).count();
        let beaten = drawn("vault", Standing::Beaten, 0).count();
        assert!(beaten > open, "beaten adds nothing");
        let (rest, _) = drawn("vault", Standing::Open, 0).finish(DYNAMIC_BUFFER);
        let (up, _) = drawn("vault", Standing::Open, ONE).finish(DYNAMIC_BUFFER);
        // the first instance's height (row 1's translation)
        assert!(up[7] > rest[7], "hover does not lift it");
    }
}
