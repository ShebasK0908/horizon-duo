use crate::{
    blocks::{BlockFacts, Fluid, RenderShape},
    world::World,
};
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub pos: [i32; 3],
    pub face: [i32; 3],
    pub point: [f64; 3],
}
pub fn clip_ray(
    origin: [f64; 3],
    dir: [f64; 3],
    reach: f64,
    world: &World,
    facts: &BlockFacts,
    outline_boxes: impl Fn([i32; 3]) -> Vec<[f64; 6]>,
) -> Option<Hit> {
    let mut pos = origin.map(|v| v.floor() as i32);
    let step = dir.map(|d| if d > 0.0 { 1 } else { -1 });
    let delta = dir.map(|d| {
        if d == 0.0 {
            f64::INFINITY
        } else {
            (1.0 / d).abs()
        }
    });
    let mut t_max = [0.0; 3];
    for i in 0..3 {
        let boundary = if dir[i] > 0.0 {
            f64::from(pos[i]) + 1.0
        } else {
            f64::from(pos[i])
        };
        t_max[i] = if dir[i] == 0.0 {
            f64::INFINITY
        } else {
            (boundary - origin[i]) / dir[i]
        };
    }
    let mut face = [0; 3];
    let mut t = 0.0;
    while t <= reach {
        let state = world.block(pos[0], pos[1], pos[2]);
        let f = facts.get(state);
        if !facts.is_air(state)
            && (f.fluid == Fluid::None
                || f.waterlogged
                || &*f.name == "minecraft:kelp"
                || f.render_shape == RenderShape::Model)
            && !matches!(
                &*f.name,
                "minecraft:water" | "minecraft:lava" | "minecraft:bubble_column"
            )
        {
            // The nearest entry into one of the outline's boxes (slab method).
            let mut best: Option<(f64, [i32; 3])> = None;
            for b in outline_boxes(pos) {
                let (mut t0, mut t1, mut axis_in, mut sign) = (0.0f64, f64::INFINITY, None, 0);
                let mut miss = false;
                for a in 0..3 {
                    if dir[a] == 0.0 {
                        miss |= origin[a] < b[a] || origin[a] > b[a + 3];
                        continue;
                    }
                    let (lo, hi) = ((b[a] - origin[a]) / dir[a], (b[a + 3] - origin[a]) / dir[a]);
                    let (near, far) = if lo < hi { (lo, hi) } else { (hi, lo) };
                    if near > t0 {
                        t0 = near;
                        axis_in = Some(a);
                        sign = if dir[a] > 0.0 { -1 } else { 1 };
                    }
                    t1 = t1.min(far);
                }
                if miss || t0 > t1 || t0 > reach {
                    continue;
                }
                let mut normal = face;
                if let Some(a) = axis_in {
                    normal = [0; 3];
                    normal[a] = sign;
                }
                if best.is_none_or(|(bt, _)| t0 < bt) {
                    best = Some((t0, normal));
                }
            }
            if let Some((t, face)) = best {
                return Some(Hit {
                    pos,
                    face,
                    point: [
                        origin[0] + dir[0] * t,
                        origin[1] + dir[1] * t,
                        origin[2] + dir[2] * t,
                    ],
                });
            }
        }
        let axis = if t_max[0] < t_max[1] {
            if t_max[0] < t_max[2] { 0 } else { 2 }
        } else if t_max[1] < t_max[2] {
            1
        } else {
            2
        };
        t = t_max[axis];
        t_max[axis] += delta[axis];
        pos[axis] += step[axis];
        face = [0; 3];
        face[axis] = -step[axis];
    }
    None
}
