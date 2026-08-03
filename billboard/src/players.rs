//! Who is watching: position and look direction of the placement's viewers.
//!
//! [`players`] hands back a snapshot — plain owned data, not live handles. That
//! is deliberate and it is what the host can honestly offer: the plugin samples
//! viewers on the server thread every few ticks and hands the result to the
//! worker the animation runs on, so there is no live world to read from here
//! anyway. One snapshot is internally consistent (a player's position and their
//! facing are from the same instant, never mixed across a [`sleep`]), and a
//! player who has walked away simply is not in the next one — no per-getter
//! `Option`.
//!
//! ```ignore
//! use billboard::prelude::*;
//!
//! // Everyone, nearest first.
//! for player in players() {
//!     log(&format!("{} at {:?}", player.name(), player.position()));
//! }
//!
//! // The four nearest the stage, within 16 blocks.
//! let front_row = players_with(
//!     Query::new()
//!         .origin(Position::new(0.0, 3.0, 0.0))
//!         .range(16.0)
//!         .limit(4),
//! );
//! ```
//!
//! Everything is in the placement's local frame, like every other
//! [`Position`] in this SDK: the host maps world coordinates and angles back
//! through the placement's origin and rotation before they cross, so an
//! animation authored around `Position::ZERO` never has to know where it was
//! spawned or which way it was turned.
//!
//! # Identity, and the cheap refresh
//!
//! Indices shift as people move, join and leave, so a vec position means
//! nothing across two calls — [`Player::name`] is the identity key. To follow
//! one player smoothly, keep the [`Player`] and call [`Player::update`] each
//! tick: one fixed-size host call keyed by name, instead of re-querying and
//! re-parsing the whole list.
//!
//! # Non-determinism
//!
//! Where people stand is not reproducible, the same way
//! [`random_nondet`](crate::random) is not. An animation that wants a
//! replayable trace must not branch on this module.
//!
//! [`sleep`]: crate::sleep
//! [`Position`]: crate::math::Position

use crate::abi::marshal::{self, RawQuery};
use crate::math::{Degrees, Offset, Position, Radians, acos, cos, sin};

/// Every viewer of this placement, nearest the origin first.
///
/// Exactly [`players_with(Query::default())`](players_with). For a
/// `per_player` placement this is the one owner; for a shared one, everybody
/// currently watching — possibly nobody, so handle the empty case.
pub fn players() -> Vec<Player> {
    players_with(Query::default())
}

/// The viewers matching `query`, filtered, sorted and limited **host-side**.
///
/// The full list can be long and most animations want a handful of it, so the
/// query crosses rather than the list: the host applies `range`, `limit` and
/// `sort` before it packs a single byte.
pub fn players_with(query: Query) -> Vec<Player> {
    parse(&marshal::players(&query.raw()))
}

/// Which order [`players_with`] reports its results in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    /// Nearest the query origin first. Ties break by name, so the order is
    /// stable for two players at the same distance.
    #[default]
    Distance,
    /// By account name, ascending.
    Name,
}

impl Sort {
    /// The integer the host reads out of the query struct.
    fn wire(self) -> i32 {
        match self {
            Sort::Distance => 0,
            Sort::Name => 1,
        }
    }
}

/// What to ask for. Every knob is optional; [`Query::default`] means "everyone,
/// nearest the placement origin first".
///
/// ```ignore
/// let watchers = players_with(Query::new().range(24.0).sort(Sort::Name));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Query {
    origin: Position,
    /// Negative is the wire's "unlimited", which is what an unset range means.
    range: f64,
    /// Zero is the wire's "unlimited", which is what an unset limit means.
    limit: usize,
    sort: Sort,
}

impl Default for Query {
    fn default() -> Query {
        Query {
            origin: Position::ZERO,
            range: -1.0,
            limit: 0,
            sort: Sort::Distance,
        }
    }
}

impl Query {
    /// An unfiltered query: everyone, nearest the placement origin first.
    pub fn new() -> Query {
        Query::default()
    }

    /// Measure distances from here instead of from `Position::ZERO` — the point
    /// of the scene people are actually meant to be looking at.
    pub fn origin(mut self, origin: Position) -> Query {
        self.origin = origin;
        self
    }

    /// Drop anyone further than `blocks` from the origin.
    ///
    /// A negative value means no limit, which is also the default.
    pub fn range(mut self, blocks: f64) -> Query {
        self.range = blocks;
        self
    }

    /// Keep at most `count` results — the first `count` in the query's own sort
    /// order, so with the default sort this is "the nearest `count`". Zero means
    /// no limit.
    pub fn limit(mut self, count: usize) -> Query {
        self.limit = count;
        self
    }

    /// Which order the results come back in.
    pub fn sort(mut self, sort: Sort) -> Query {
        self.sort = sort;
        self
    }

    /// The 40 packed bytes the host reads.
    fn raw(&self) -> RawQuery {
        RawQuery {
            origin_x: self.origin.x,
            origin_y: self.origin.y,
            origin_z: self.origin.z,
            range: self.range,
            // A limit past `i32::MAX` is "everyone" by any sane reading, and
            // saturating says that without a kill over an absurd number.
            limit: i32::try_from(self.limit).unwrap_or(i32::MAX),
            sort: self.sort.wire(),
        }
    }
}

/// One viewer, as of the snapshot this came from.
///
/// Owned data with no host handle behind it: the values are frozen at the
/// moment the host sampled them, and [`update`](Player::update) is how a held
/// `Player` catches up. Positions and angles are placement-local.
#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    name: String,
    position: Position,
    eye_height: f64,
    yaw: f64,
    pitch: f64,
}

impl Player {
    /// The account name — what `/bb env`'s `bb.player` and the placement's
    /// whitelist entries use, and the only identity that survives a refresh.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where they are standing: the **feet**, in placement-local coordinates.
    pub fn position(&self) -> Position {
        self.position
    }

    /// Where they are looking *from*: the feet plus the host's eye height,
    /// which already accounts for sneaking and the like.
    pub fn eye_position(&self) -> Position {
        self.position + Offset::new(0.0, self.eye_height, 0.0)
    }

    /// Horizontal rotation in degrees, vanilla's convention (0 faces `+Z`,
    /// increasing turns towards `−X`), measured in the placement's frame.
    pub fn yaw(&self) -> f64 {
        self.yaw
    }

    /// Vertical rotation in degrees, vanilla's convention: `−90` is straight up,
    /// `0` is level, `90` is straight down.
    pub fn pitch(&self) -> f64 {
        self.pitch
    }

    /// The unit vector they are looking along, derived from yaw and pitch:
    ///
    /// ```text
    /// x = −sin(yaw) · cos(pitch)
    /// y = −sin(pitch)
    /// z =  cos(yaw) · cos(pitch)
    /// ```
    ///
    /// Guest-side arithmetic — the host sends angles, not a vector — but the
    /// trigonometry goes through the [math kernel](crate::math::sin), so the
    /// numbers are the host's own and identical on both targets.
    pub fn facing(&self) -> Offset {
        let yaw = Radians::from(Degrees::new(self.yaw)).value();
        let pitch = Radians::from(Degrees::new(self.pitch)).value();
        let cos_pitch = cos(pitch);
        Offset::new(-sin(yaw) * cos_pitch, -sin(pitch), cos(yaw) * cos_pitch)
    }

    /// How far off `target` their gaze is, in degrees: `0` is looking straight
    /// at it, `180` is directly away from it. The angle between
    /// [`facing`](Player::facing) and the direction from
    /// [`eye_position`](Player::eye_position) to `target`.
    ///
    /// This is the "is anyone watching?" check — compare against a cone
    /// half-angle rather than an exact value.
    ///
    /// A `target` sitting exactly on the eye has no direction to it, and reads
    /// as `0`: it is as looked-at as a point can be.
    pub fn looking_toward(&self, target: Position) -> f64 {
        let to = target - self.eye_position();
        let distance = to.length();
        if distance == 0.0 {
            return 0.0;
        }
        // `facing` is a unit vector, so dividing by the target's distance alone
        // normalizes the whole dot product. Clamped because a dot of two
        // *nearly* unit vectors can land a hair outside [−1, 1] and `acos` of
        // 1.0000000000000002 is NaN.
        let cosine = (self.facing().dot(to) / distance).clamp(-1.0, 1.0);
        Degrees::from(Radians::new(acos(cosine))).value()
    }

    /// Refresh this player from the host's latest snapshot, in place.
    ///
    /// `true` if they are still a viewer (every field is now current), `false`
    /// if they are not — in which case the fields are left exactly as they were,
    /// so a follow animation can coast on the last known position instead of
    /// snapping to the origin.
    ///
    /// One fixed-size host call, which is what makes a per-tick follow cheap
    /// next to re-running [`players`].
    pub fn update(&mut self) -> bool {
        match marshal::player_update(&self.name) {
            Some([x, y, z, eye_height, yaw, pitch]) => {
                self.position = Position::new(x, y, z);
                self.eye_height = eye_height;
                self.yaw = yaw;
                self.pitch = pitch;
                true
            }
            None => false,
        }
    }
}

/// Parse the host's snapshot blob, or die saying how the two sides disagree.
///
/// The host is trusted, so this is diagnosis rather than defence: a short,
/// mis-counted or non-UTF-8 blob means the wire formats have drifted apart, and
/// that should stop the animation loudly rather than serve half a list.
fn parse(blob: &[u8]) -> Vec<Player> {
    try_parse(blob).unwrap_or_else(|e| panic!("{e}"))
}

/// Blob layout, every integer a little-endian `u32`: a player count, then per
/// player a name length, the name's UTF-8 bytes, and six `f64`s — `x`, `y`, `z`
/// (feet), `eye_height`, `yaw`, `pitch`.
fn try_parse(blob: &[u8]) -> Result<Vec<Player>, String> {
    // An empty snapshot is an empty blob: `players_len` returned 0 and the
    // second host call never happened, so there is not even a count to read.
    if blob.is_empty() {
        return Ok(Vec::new());
    }
    let mut reader = Reader { blob, at: 0 };
    let count = reader.u32("player count")? as usize;
    // `with_capacity(count)` on a bogus count would reserve gigabytes before the
    // read failed; the cap keeps a disagreement cheap.
    let mut players = Vec::with_capacity(count.min(256));
    for i in 0..count {
        let name = reader.string(&format!("name of player {i}"))?;
        players.push(Player {
            position: Position::new(
                reader.f64(&format!("x of player {i}"))?,
                reader.f64(&format!("y of player {i}"))?,
                reader.f64(&format!("z of player {i}"))?,
            ),
            eye_height: reader.f64(&format!("eye height of player {i}"))?,
            yaw: reader.f64(&format!("yaw of player {i}"))?,
            pitch: reader.f64(&format!("pitch of player {i}"))?,
            name,
        });
    }
    if reader.at != blob.len() {
        return Err(format!(
            "player snapshot has {} trailing bytes after {count} players",
            blob.len() - reader.at
        ));
    }
    Ok(players)
}

/// A cursor over the blob. Every read either advances or reports exactly what it
/// was short of.
struct Reader<'a> {
    blob: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    /// `wrapping_add` throughout: a bogus length from a disagreeing host must
    /// come back as "the blob is too short", not as an arithmetic overflow — and
    /// on wasm32 a `u32` length can wrap `usize`.
    fn take(&mut self, n: usize, what: &str) -> Result<&[u8], String> {
        let end = self.at.wrapping_add(n);
        let bytes = self.blob.get(self.at..end).ok_or_else(|| {
            format!(
                "player snapshot ends mid-{what}: need {n} bytes at offset {}, blob is {} bytes",
                self.at,
                self.blob.len()
            )
        })?;
        self.at = end;
        Ok(bytes)
    }

    fn u32(&mut self, what: &str) -> Result<u32, String> {
        let bytes = self.take(4, what)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn f64(&mut self, what: &str) -> Result<f64, String> {
        let bytes = self.take(8, what)?;
        let mut raw = [0u8; 8];
        raw.copy_from_slice(bytes);
        Ok(f64::from_le_bytes(raw))
    }

    fn string(&mut self, what: &str) -> Result<String, String> {
        let len = self.u32(what)? as usize;
        let bytes = self.take(len, what)?;
        core::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|e| format!("player snapshot {what} is not UTF-8: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every blob below is written out by hand, byte by byte, so a bug in a
    /// serializer could never cancel out against a bug in the parser. `u32`
    /// lengths are spelled as their four little-endian bytes, and `f64`s go in
    /// through `to_le_bytes` on a literal — the one thing worth trusting a
    /// library for, since hand-writing an IEEE-754 mantissa proves nothing about
    /// the reader.
    fn blob(count: u32, players: &[(&str, [f64; 6])]) -> Vec<u8> {
        let mut bytes = count.to_le_bytes().to_vec();
        for (name, values) in players {
            bytes.extend((name.len() as u32).to_le_bytes());
            bytes.extend(name.as_bytes());
            for value in values {
                bytes.extend(value.to_le_bytes());
            }
        }
        bytes
    }

    fn player(name: &str, values: [f64; 6]) -> Player {
        Player {
            name: name.to_owned(),
            position: Position::new(values[0], values[1], values[2]),
            eye_height: values[3],
            yaw: values[4],
            pitch: values[5],
        }
    }

    /// Roughly: within a hair of the exact value. `cos(π/2)` is 6.1e-17 rather
    /// than 0, which is the size of error these comparisons must tolerate.
    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    // --- The wire: query struct out, snapshot blob back. ---

    #[test]
    fn the_query_struct_is_the_forty_bytes_the_host_reads() {
        assert_eq!(core::mem::size_of::<RawQuery>(), 40);

        // A query with a distinct, hand-checkable value in every field, laid
        // out against the bytes the host will read: three origin f64s, the
        // range, then the two i32s. 1.0 is 0x3FF0_0000_0000_0000, 2.0 is
        // 0x4000…, 3.0 is 0x4008…, 4.0 is 0x4010… — little-endian, so the
        // leading byte is the low one and the last two carry the exponent.
        let raw = Query::new()
            .origin(Position::new(1.0, 2.0, 3.0))
            .range(4.0)
            .limit(7)
            .sort(Sort::Name)
            .raw();
        let bytes: [u8; 40] = unsafe { core::mem::transmute(raw) };
        assert_eq!(
            bytes,
            [
                0, 0, 0, 0, 0, 0, 0xF0, 0x3F, // origin_x = 1.0
                0, 0, 0, 0, 0, 0, 0x00, 0x40, // origin_y = 2.0
                0, 0, 0, 0, 0, 0, 0x08, 0x40, // origin_z = 3.0
                0, 0, 0, 0, 0, 0, 0x10, 0x40, // range    = 4.0
                7, 0, 0, 0, // limit = 7
                1, 0, 0, 0, // sort  = Sort::Name
            ]
        );
    }

    #[test]
    fn an_unset_query_asks_for_everyone_from_the_placement_origin() {
        let raw = Query::default().raw();
        assert_eq!(raw.origin_x, 0.0);
        assert_eq!(raw.origin_y, 0.0);
        assert_eq!(raw.origin_z, 0.0);
        // Negative range and zero limit are the wire's two spellings of
        // "unlimited"; distance-ascending is sort 0.
        assert!(raw.range < 0.0);
        assert_eq!(raw.limit, 0);
        assert_eq!(raw.sort, 0);
    }

    #[test]
    fn an_absurd_limit_saturates_rather_than_killing() {
        assert_eq!(Query::new().limit(usize::MAX).raw().limit, i32::MAX);
    }

    #[test]
    fn an_empty_snapshot_is_an_empty_blob() {
        // `players_len` returning 0 skips the second call entirely, so there is
        // no count to read — and a bare zero count parses to the same thing.
        assert_eq!(try_parse(&[]), Ok(Vec::new()));
        assert_eq!(try_parse(&[0, 0, 0, 0]), Ok(Vec::new()));
    }

    #[test]
    fn one_player_round_trips_every_field() {
        let bytes = blob(1, &[("Steve", [1.5, 64.0, -2.5, 1.62, 90.0, -45.0])]);
        // count(4) + name_len(4) + "Steve"(5) + 6 f64s(48) = 61 bytes.
        assert_eq!(bytes.len(), 61);

        let parsed = try_parse(&bytes).expect("one player parses");
        assert_eq!(parsed.len(), 1);
        let p = &parsed[0];
        assert_eq!(p.name(), "Steve");
        assert_eq!(p.position(), Position::new(1.5, 64.0, -2.5));
        assert_eq!(p.eye_height, 1.62);
        assert_eq!(p.yaw(), 90.0);
        assert_eq!(p.pitch(), -45.0);
    }

    #[test]
    fn two_players_parse_in_order_including_a_multibyte_name() {
        // "Ωmega" is six bytes for five characters, which is the case a
        // char-counting length would get wrong.
        let names = ("Alex", "Ωmega");
        assert_eq!(names.1.len(), 6);
        assert_eq!(names.1.chars().count(), 5);

        let bytes = blob(
            2,
            &[
                (names.0, [0.0, 0.0, 0.0, 1.62, 0.0, 0.0]),
                (names.1, [3.0, 1.0, -4.0, 1.27, 180.0, 12.5]),
            ],
        );
        let parsed = try_parse(&bytes).expect("two players parse");
        assert_eq!(
            parsed,
            vec![
                player("Alex", [0.0, 0.0, 0.0, 1.62, 0.0, 0.0]),
                player("Ωmega", [3.0, 1.0, -4.0, 1.27, 180.0, 12.5]),
            ]
        );
    }

    #[test]
    fn a_truncated_blob_says_what_it_was_short_of() {
        let mut bytes = blob(1, &[("Steve", [1.0, 2.0, 3.0, 1.62, 0.0, 0.0])]);
        bytes.truncate(bytes.len() - 1);
        let error = try_parse(&bytes).expect_err("a short blob must not parse");
        assert!(error.contains("pitch of player 0"), "{error}");
    }

    #[test]
    fn a_blob_with_bytes_left_over_is_a_disagreement_not_a_shrug() {
        let mut bytes = blob(1, &[("Steve", [1.0, 2.0, 3.0, 1.62, 0.0, 0.0])]);
        bytes.push(0);
        let error = try_parse(&bytes).expect_err("trailing bytes must not parse");
        assert!(error.contains("1 trailing bytes"), "{error}");
    }

    #[test]
    #[should_panic(expected = "player snapshot ends mid-")]
    fn a_malformed_snapshot_kills_the_animation() {
        // The count says one player, the blob stops right there.
        let _ = parse(&[1, 0, 0, 0]);
    }

    // --- The derivations. ---

    #[test]
    fn the_eye_sits_exactly_the_eye_height_above_the_feet() {
        let p = player("Steve", [1.5, 64.0, -2.5, 1.62, 0.0, 0.0]);
        assert_eq!(p.position(), Position::new(1.5, 64.0, -2.5));
        assert_eq!(p.eye_position(), Position::new(1.5, 65.62, -2.5));
        // Only Y moves.
        assert_eq!(p.eye_position().x, p.position().x);
        assert_eq!(p.eye_position().z, p.position().z);
    }

    #[test]
    fn facing_matches_vanilla_at_the_cardinal_angles() {
        // Yaw 0 is south, +Z — the direction the SDK's assumed viewer looks.
        let south = player("a", [0.0; 6]).facing();
        close(south.x, 0.0);
        close(south.y, 0.0);
        close(south.z, 1.0);

        // Yaw 90 turns towards −X (west), which is why the x term is negated.
        let west = player("a", [0.0, 0.0, 0.0, 1.62, 90.0, 0.0]).facing();
        close(west.x, -1.0);
        close(west.y, 0.0);
        close(west.z, 0.0);

        // Yaw 180 is north, −Z.
        let north = player("a", [0.0, 0.0, 0.0, 1.62, 180.0, 0.0]).facing();
        close(north.z, -1.0);

        // Pitch −90 is straight up, pitch 90 straight down.
        let up = player("a", [0.0, 0.0, 0.0, 1.62, 0.0, -90.0]).facing();
        close(up.x, 0.0);
        close(up.y, 1.0);
        close(up.z, 0.0);
        close(player("a", [0.0, 0.0, 0.0, 1.62, 0.0, 90.0]).facing().y, -1.0);
    }

    #[test]
    fn facing_is_a_unit_vector_at_an_awkward_angle() {
        let odd = player("a", [0.0, 0.0, 0.0, 1.62, 37.0, -23.0]).facing();
        close(odd.length(), 1.0);
        // 45 degrees of yaw with no pitch splits the horizontal plane evenly,
        // and sin(45°) = cos(45°) = √2/2.
        let diagonal = player("a", [0.0, 0.0, 0.0, 1.62, 45.0, 0.0]).facing();
        let half_root_two = 0.5f64 * 2.0f64.sqrt();
        close(diagonal.x, -half_root_two);
        close(diagonal.z, half_root_two);
    }

    #[test]
    fn looking_toward_is_zero_straight_ahead_and_one_eighty_behind() {
        // Feet at the origin, eyes 1.62 up, looking south (+Z).
        let p = player("a", [0.0, 0.0, 0.0, 1.62, 0.0, 0.0]);
        close(p.looking_toward(Position::new(0.0, 1.62, 10.0)), 0.0);
        close(p.looking_toward(Position::new(0.0, 1.62, -10.0)), 180.0);
        // Straight up from the eye, while looking level, is a right angle.
        close(p.looking_toward(Position::new(0.0, 11.62, 0.0)), 90.0);
        // And a 45-degree offset in the horizontal plane reads as 45.
        close(p.looking_toward(Position::new(5.0, 1.62, 5.0)), 45.0);
    }

    #[test]
    fn looking_toward_measures_from_the_eye_not_the_feet() {
        let p = player("a", [0.0, 0.0, 0.0, 1.62, 0.0, 0.0]);
        // A point level with the *feet*, one block south: from the eye it is
        // below the horizon, so a level gaze is off by atan(1.62 / 1.0).
        let angle = p.looking_toward(Position::new(0.0, 0.0, 1.0));
        close(angle, 1.62f64.atan2(1.0).to_degrees());
        // Nothing about that answer is zero, which is what it would be if the
        // measurement started at the feet.
        assert!(angle > 50.0, "{angle}");
    }

    #[test]
    fn a_target_on_the_eye_itself_reads_as_looked_at() {
        let p = player("a", [0.0, 0.0, 0.0, 1.62, 0.0, 0.0]);
        assert_eq!(p.looking_toward(p.eye_position()), 0.0);
    }
}
