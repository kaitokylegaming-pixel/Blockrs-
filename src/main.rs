use macroquad::prelude::*;
use std::collections::HashSet;

const W: i32 = 48; // world size in blocks (x)
const D: i32 = 48; // (z)
const H: i32 = 24; // (y)
const CS: i32 = 16; // chunk size
const NC: i32 = W / CS; // chunks per side

// Block ids are 1..=5, index into this palette (0 = air).
const PALETTE: [(&str, [f32; 3]); 5] = [
    ("Grass", [0.35, 0.70, 0.25]),
    ("Dirt", [0.55, 0.38, 0.22]),
    ("Stone", [0.50, 0.50, 0.52]),
    ("Wood", [0.72, 0.55, 0.30]),
    ("Brick", [0.70, 0.30, 0.25]),
];

// (neighbour direction, 4 corners of the face, brightness)
const FACES: [([i32; 3], [[f32; 3]; 4], f32); 6] = [
    ([1, 0, 0], [[1., 0., 0.], [1., 1., 0.], [1., 1., 1.], [1., 0., 1.]], 0.80),
    ([-1, 0, 0], [[0., 0., 1.], [0., 1., 1.], [0., 1., 0.], [0., 0., 0.]], 0.80),
    ([0, 1, 0], [[0., 1., 0.], [0., 1., 1.], [1., 1., 1.], [1., 1., 0.]], 1.00),
    ([0, -1, 0], [[0., 0., 1.], [0., 0., 0.], [1., 0., 0.], [1., 0., 1.]], 0.50),
    ([0, 0, 1], [[1., 0., 1.], [1., 1., 1.], [0., 1., 1.], [0., 0., 1.]], 0.65),
    ([0, 0, -1], [[0., 0., 0.], [0., 1., 0.], [1., 1., 0.], [1., 0., 0.]], 0.65),
];

type Cell = (i32, i32, i32);

struct World {
    blocks: Vec<u8>,
    meshes: Vec<Vec<Mesh>>, // one entry per chunk column
}

impl World {
    fn new() -> Self {
        let mut blocks = vec![0u8; (W * D * H) as usize];
        for z in 0..D {
            for x in 0..W {
                let h = (6.0 + ((x as f32 * 0.25).sin() + (z as f32 * 0.2).cos()) * 2.0) as i32;
                for y in 0..=h {
                    let id = if y == h {
                        1
                    } else if y >= h - 3 {
                        2
                    } else {
                        3
                    };
                    blocks[Self::idx(x, y, z)] = id;
                }
            }
        }
        let mut w = World { blocks, meshes: Vec::new() };
        for cz in 0..NC {
            for cx in 0..NC {
                let m = w.build_chunk(cx, cz);
                w.meshes.push(m);
            }
        }
        w
    }

    fn idx(x: i32, y: i32, z: i32) -> usize {
        ((y * D + z) * W + x) as usize
    }

    fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0 || y < 0 || z < 0 || x >= W || y >= H || z >= D {
            0
        } else {
            self.blocks[Self::idx(x, y, z)]
        }
    }

    // Solid for collision and face culling. Below the world counts as solid.
    fn solid(&self, x: i32, y: i32, z: i32) -> bool {
        y < 0 || self.get(x, y, z) != 0
    }

    fn set(&mut self, x: i32, y: i32, z: i32, id: u8) {
        if x < 0 || y < 0 || z < 0 || x >= W || y >= H || z >= D {
            return;
        }
        self.blocks[Self::idx(x, y, z)] = id;
        let (cx, cz) = (x / CS, z / CS);
        let mut todo = vec![(cx, cz)];
        if x % CS == 0 && cx > 0 {
            todo.push((cx - 1, cz));
        }
        if x % CS == CS - 1 && cx < NC - 1 {
            todo.push((cx + 1, cz));
        }
        if z % CS == 0 && cz > 0 {
            todo.push((cx, cz - 1));
        }
        if z % CS == CS - 1 && cz < NC - 1 {
            todo.push((cx, cz + 1));
        }
        for (a, b) in todo {
            let m = self.build_chunk(a, b);
            self.meshes[(b * NC + a) as usize] = m;
        }
    }

    // Only visible faces are emitted, so a chunk is a handful of triangles.
    fn build_chunk(&self, cx: i32, cz: i32) -> Vec<Mesh> {
        let mut out = Vec::new();
        let mut vs: Vec<Vertex> = Vec::new();
        let mut is: Vec<u16> = Vec::new();
        for y in 0..H {
            for z in cz * CS..(cz + 1) * CS {
                for x in cx * CS..(cx + 1) * CS {
                    let id = self.get(x, y, z);
                    if id == 0 {
                        continue;
                    }
                    let c = PALETTE[(id - 1) as usize].1;
                    let v = 1.0 - ((x * 7 + y * 13 + z * 3) % 5) as f32 * 0.03;
                    for (dir, corners, shade) in FACES.iter() {
                        if self.solid(x + dir[0], y + dir[1], z + dir[2]) {
                            continue;
                        }
                        let col = Color::new(c[0] * shade * v, c[1] * shade * v, c[2] * shade * v, 1.0);
                        let base = vs.len() as u16;
                        for k in corners.iter() {
                            vs.push(Vertex::new(
                                x as f32 + k[0],
                                y as f32 + k[1],
                                z as f32 + k[2],
                                0.0,
                                0.0,
                                col,
                            ));
                        }
                        is.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                    }
                    // u16 indices: flush well before 65535 vertices
                    if vs.len() > 60000 {
                        out.push(Mesh {
                            vertices: std::mem::take(&mut vs),
                            indices: std::mem::take(&mut is),
                            texture: None,
                        });
                    }
                }
            }
        }
        if !vs.is_empty() {
            out.push(Mesh { vertices: vs, indices: is, texture: None });
        }
        out
    }

    fn collides(&self, p: Vec3) -> bool {
        let (r, h) = (0.3, 1.7);
        for y in (p.y.floor() as i32)..=((p.y + h).floor() as i32) {
            for z in ((p.z - r).floor() as i32)..=((p.z + r).floor() as i32) {
                for x in ((p.x - r).floor() as i32)..=((p.x + r).floor() as i32) {
                    if self.solid(x, y, z) {
                        return true;
                    }
                }
            }
        }
        false
    }

    // Returns (block hit, last empty cell before it)
    fn raycast(&self, o: Vec3, d: Vec3) -> Option<(Cell, Cell)> {
        let cell = |p: Vec3| (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let mut prev = cell(o);
        let mut t = 0.0;
        while t < 6.0 {
            let c = cell(o + d * t);
            if self.get(c.0, c.1, c.2) != 0 {
                return Some((c, prev));
            }
            prev = c;
            t += 0.03;
        }
        None
    }
}

fn player_overlaps(pos: Vec3, c: Cell) -> bool {
    let (r, h) = (0.3, 1.7);
    pos.x + r > c.0 as f32
        && pos.x - r < c.0 as f32 + 1.0
        && pos.y + h > c.1 as f32
        && pos.y < c.1 as f32 + 1.0
        && pos.z + r > c.2 as f32
        && pos.z - r < c.2 as f32 + 1.0
}

#[macroquad::main("BlockRS")]
async fn main() {
    let mut world = World::new();
    let spawn = vec3(W as f32 / 2.0, H as f32 - 2.0, D as f32 / 2.0);
    let mut pos = spawn;
    let mut vel_y = 0.0f32;
    let mut on_ground = false;
    let (mut yaw, mut pitch) = (0.0f32, -0.3f32);
    let mut placing = false;
    let mut block_id: u8 = 3;

    // touch state
    let mut seen: HashSet<u64> = HashSet::new();
    let mut move_id: Option<u64> = None;
    let mut move_origin = Vec2::ZERO;
    let mut stick = Vec2::ZERO;
    let mut look_id: Option<u64> = None;
    let mut look_last = Vec2::ZERO;
    let mut look_start = Vec2::ZERO;
    let mut look_t0 = 0.0f64;

    loop {
        let dt = get_frame_time().min(0.05);
        let (sw, sh) = (screen_width(), screen_height());
        let u = sh * 0.12;
        let stick_r = sh * 0.15;
        let jump_c = vec2(sw - u * 1.4, sh - u * 1.4);
        let jump_r = u * 0.95;
        let mode_btn = Rect::new(sw - u * 3.4, u * 0.3, u * 3.1, u * 0.9);
        let block_btn = Rect::new(sw - u * 3.4, u * 1.4, u * 3.1, u * 0.9);
        let mut jump_req = false;
        let mut tap = false;

        // ---- touch input ----
        for t in touches() {
            let p = t.position;
            if matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                if move_id == Some(t.id) {
                    move_id = None;
                    stick = Vec2::ZERO;
                }
                if look_id == Some(t.id) {
                    if (p - look_start).length() < 25.0 && get_time() - look_t0 < 0.3 {
                        tap = true;
                    }
                    look_id = None;
                }
                seen.remove(&t.id);
                continue;
            }
            if seen.insert(t.id) {
                // a new finger just touched down
                if (p - jump_c).length() < jump_r {
                    jump_req = true;
                } else if mode_btn.contains(p) {
                    placing = !placing;
                } else if block_btn.contains(p) {
                    block_id = block_id % 5 + 1;
                } else if p.x < sw * 0.4 && move_id.is_none() {
                    move_id = Some(t.id);
                    move_origin = p;
                } else if look_id.is_none() {
                    look_id = Some(t.id);
                    look_last = p;
                    look_start = p;
                    look_t0 = get_time();
                }
            } else if move_id == Some(t.id) {
                stick = ((p - move_origin) / stick_r).clamp_length_max(1.0);
            } else if look_id == Some(t.id) {
                let d = p - look_last;
                look_last = p;
                yaw += d.x * 0.005;
                pitch = (pitch - d.y * 0.005).clamp(-1.5, 1.5);
            }
        }

        // ---- keyboard (handy for testing on a PC) ----
        let mut mv = vec2(stick.x, -stick.y);
        if is_key_down(KeyCode::W) { mv.y += 1.0; }
        if is_key_down(KeyCode::S) { mv.y -= 1.0; }
        if is_key_down(KeyCode::D) { mv.x += 1.0; }
        if is_key_down(KeyCode::A) { mv.x -= 1.0; }
        if is_key_down(KeyCode::Left) { yaw -= 2.0 * dt; }
        if is_key_down(KeyCode::Right) { yaw += 2.0 * dt; }
        if is_key_down(KeyCode::Up) { pitch = (pitch + 1.5 * dt).min(1.5); }
        if is_key_down(KeyCode::Down) { pitch = (pitch - 1.5 * dt).max(-1.5); }
        if is_key_pressed(KeyCode::Space) { jump_req = true; }
        if is_key_pressed(KeyCode::G) { placing = !placing; }
        if is_key_pressed(KeyCode::F) { tap = true; }
        let mv = mv.clamp_length_max(1.0);

        // ---- movement and physics ----
        let fwd = vec3(yaw.cos(), 0.0, yaw.sin());
        let right = vec3(-yaw.sin(), 0.0, yaw.cos());
        let wish = (right * mv.x + fwd * mv.y) * 4.5 * dt;
        let try_x = vec3(pos.x + wish.x, pos.y, pos.z);
        if !world.collides(try_x) { pos = try_x; }
        let try_z = vec3(pos.x, pos.y, pos.z + wish.z);
        if !world.collides(try_z) { pos = try_z; }
        pos.x = pos.x.clamp(0.4, W as f32 - 0.4);
        pos.z = pos.z.clamp(0.4, D as f32 - 0.4);

        if jump_req && on_ground { vel_y = 8.0; }
        vel_y -= 25.0 * dt;
        on_ground = false;
        let try_y = vec3(pos.x, pos.y + vel_y * dt, pos.z);
        if world.collides(try_y) {
            if vel_y < 0.0 {
                let snapped = vec3(pos.x, try_y.y.floor() + 1.0, pos.z);
                if !world.collides(snapped) { pos = snapped; }
                on_ground = true;
            }
            vel_y = 0.0;
        } else {
            pos = try_y;
        }
        if pos.y < -10.0 { pos = spawn; vel_y = 0.0; }

        // ---- aim, break / place ----
        let eye = pos + vec3(0.0, 1.6, 0.0);
        let look = vec3(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos());
        let hit = world.raycast(eye, look);
        if tap {
            if let Some((h, prev)) = hit {
                if placing {
                    if !player_overlaps(pos, prev) {
                        world.set(prev.0, prev.1, prev.2, block_id);
                    }
                } else {
                    world.set(h.0, h.1, h.2, 0);
                }
            }
        }

        // ---- draw 3D ----
        clear_background(Color::new(0.53, 0.81, 0.92, 1.0));
        set_camera(&Camera3D {
            position: eye,
            target: eye + look,
            up: vec3(0.0, 1.0, 0.0),
            fovy: 70f32.to_radians(),
            ..Default::default()
        });
        for chunk in &world.meshes {
            for m in chunk {
                draw_mesh(m);
            }
        }
        if let Some((h, _)) = hit {
            draw_cube_wires(
                vec3(h.0 as f32 + 0.5, h.1 as f32 + 0.5, h.2 as f32 + 0.5),
                vec3(1.01, 1.01, 1.01),
                BLACK,
            );
        }

        // ---- draw UI ----
        set_default_camera();
        let (cx, cy) = (sw / 2.0, sh / 2.0);
        draw_line(cx - 12.0, cy, cx + 12.0, cy, 2.0, WHITE);
        draw_line(cx, cy - 12.0, cx, cy + 12.0, 2.0, WHITE);

        if move_id.is_some() {
            draw_circle_lines(move_origin.x, move_origin.y, stick_r, 3.0, Color::new(1.0, 1.0, 1.0, 0.5));
            let k = move_origin + stick * stick_r;
            draw_circle(k.x, k.y, stick_r * 0.35, Color::new(1.0, 1.0, 1.0, 0.4));
        }

        draw_circle(jump_c.x, jump_c.y, jump_r, Color::new(1.0, 1.0, 1.0, 0.25));
        draw_text("JUMP", jump_c.x - u * 0.5, jump_c.y + u * 0.2, u * 0.6, WHITE);

        let shade = Color::new(0.0, 0.0, 0.0, 0.45);
        draw_rectangle(mode_btn.x, mode_btn.y, mode_btn.w, mode_btn.h, shade);
        draw_text(
            if placing { "PLACE" } else { "BREAK" },
            mode_btn.x + u * 0.2,
            mode_btn.y + u * 0.65,
            u * 0.6,
            WHITE,
        );

        let (name, c) = PALETTE[(block_id - 1) as usize];
        draw_rectangle(block_btn.x, block_btn.y, block_btn.w, block_btn.h, shade);
        draw_rectangle(
            block_btn.x + u * 0.1,
            block_btn.y + u * 0.1,
            u * 0.7,
            u * 0.7,
            Color::new(c[0], c[1], c[2], 1.0),
        );
        draw_text(name, block_btn.x + u * 0.95, block_btn.y + u * 0.65, u * 0.55, WHITE);

        draw_text(&format!("{} fps", get_fps()), 10.0, 30.0, 30.0, WHITE);

        next_frame().await;
    }
                                               }
