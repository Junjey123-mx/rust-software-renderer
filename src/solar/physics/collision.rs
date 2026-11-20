//! Collision of the spacecraft with the bodies: each physics substep's
//! segment is swept against a sphere (body radius plus the ship radius
//! plus a margin) in the body's moving frame. On contact the ship sits on
//! that sphere and loses the velocity that goes in, keeping the tangential
//! part: no bounce, no penetration.

use crate::solar::ShipState;
use crate::solar::planet::HeightField;
use raylib::math::Vector3;

/// Room left between the ship's bounding sphere and a body's surface.
pub const COLLISION_MARGIN: f32 = 0.05;

/// A body as the ship sees it this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionShape {
    /// Center at the start of the frame.
    pub center: Vector3,
    /// Body radius plus the ship radius plus `COLLISION_MARGIN`.
    pub radius: f32,
    /// World velocity of the center during the frame.
    pub velocity: Vector3,
}

impl CollisionShape {
    /// The body of `body_radius` at `center`, moving at `velocity`, for a
    /// ship of `ship_radius`.
    pub fn new(center: Vector3, body_radius: f32, ship_radius: f32, velocity: Vector3) -> Self {
        Self {
            center,
            radius: body_radius + ship_radius + COLLISION_MARGIN,
            velocity,
        }
    }
}

/// Where a swept segment meets a shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    /// Fraction of the segment, in `[0, 1]` (0 when it starts inside).
    pub t: f32,
    /// Unit outward normal at the contact.
    pub normal: Vector3,
}

fn unit(v: Vector3) -> Option<Vector3> {
    let len = v.length();
    (len.is_finite() && len > 1e-6).then(|| v / len)
}

/// First point of the segment `from -> to` (relative to the shape's center)
/// on a sphere of `radius` around the origin; a start inside is a contact
/// at `t = 0` along the nearest normal. `None` when it stays outside.
pub fn sweep(from: Vector3, to: Vector3, radius: f32) -> Option<Contact> {
    let up = Vector3::new(0.0, 1.0, 0.0);
    if from.length() < radius {
        let normal = unit(from).or_else(|| unit(to)).unwrap_or(up);
        return Some(Contact { t: 0.0, normal });
    }
    let d = to - from;
    let a = d.dot(d);
    if !(a.is_finite() && a > 1e-12) {
        return None;
    }
    let b = from.dot(d);
    let c = from.dot(from) - radius * radius;
    let disc = b * b - a * c;
    if disc < 0.0 {
        return None;
    }
    let t = (-b - disc.sqrt()) / a;
    if !(0.0..=1.0).contains(&t) {
        return None;
    }
    let normal = unit(from + d * t).unwrap_or(up);
    Some(Contact { t, normal })
}

/// Stops the ship at the contact: on the shape's sphere along the normal
/// (around `center`, the shape's center now), without the velocity that
/// goes into the body relative to its motion; the tangential part stays.
pub fn resolve(ship: &mut ShipState, contact: &Contact, shape: &CollisionShape, center: Vector3) {
    ship.position = center + contact.normal * shape.radius;
    let inward = (ship.velocity - shape.velocity).dot(contact.normal);
    if inward < 0.0 {
        ship.velocity -= contact.normal * inward;
    }
}

/// One physics substep of `h` seconds, `elapsed` into the frame: the ship
/// moved from `from` to its current position. Checks every shape in its
/// own moving frame and resolves the contacts.
pub fn collide(
    ship: &mut ShipState,
    from: Vector3,
    shapes: &[CollisionShape],
    elapsed: f32,
    h: f32,
) {
    for shape in shapes {
        let start = shape.center + shape.velocity * elapsed;
        let end = start + shape.velocity * h;
        if let Some(contact) = sweep(from - start, ship.position - end, shape.radius) {
            resolve(ship, &contact, shape, end);
        }
    }
}

/// Surface of a body around its center: its radius in each direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SurfaceSampler {
    Sphere {
        radius: f32,
    },
    /// The relief of `field` (the same field the mesh is built from) over
    /// a planet of radius `base`.
    Field {
        base: f32,
        field: HeightField,
    },
}

impl SurfaceSampler {
    /// Distance from the center to the ground along the unit `dir`.
    pub fn radius_at(&self, dir: Vector3) -> f32 {
        match *self {
            Self::Sphere { radius } => radius,
            Self::Field { base, field } => base * field.surface_radius(dir),
        }
    }

    /// Largest `radius_at`: the bound of the broad phase.
    pub fn max_radius(&self) -> f32 {
        match *self {
            Self::Sphere { radius } => radius,
            Self::Field { base, field } => base * (1.0 + field.max_height()),
        }
    }
}

/// `point` raised along the radial to at least `clearance` above the
/// ground `radius_at` of a body at `center` whose ground never reaches past
/// `max_radius` (broad phase: farther points are left as they are). Used
/// for the ship and for the follow camera's eye.
pub fn lift_above<S: Fn(Vector3) -> f32>(
    point: Vector3,
    center: Vector3,
    max_radius: f32,
    clearance: f32,
    radius_at: S,
) -> Vector3 {
    let offset = point - center;
    let d = offset.length();
    if !d.is_finite() || d > max_radius + clearance {
        return point;
    }
    let normal = unit(offset).unwrap_or(Vector3::new(0.0, 1.0, 0.0));
    let floor = radius_at(normal) + clearance;
    if d < floor {
        center + normal * floor
    } else {
        point
    }
}

/// Keeps the ship's bounding sphere (plus `COLLISION_MARGIN`) above the
/// ground of a body at `center` moving at `body_velocity`: narrow phase
/// with `radius_at` in the ship's direction once inside `max_radius`. The
/// velocity into the ground, relative to the body, is removed.
pub fn collide_with<S: Fn(Vector3) -> f32>(
    ship: &mut ShipState,
    center: Vector3,
    body_velocity: Vector3,
    max_radius: f32,
    radius_at: S,
) {
    let clearance = ship.radius() + COLLISION_MARGIN;
    let lifted = lift_above(ship.position, center, max_radius, clearance, radius_at);
    if lifted != ship.position {
        let normal = (lifted - center).normalized();
        ship.position = lifted;
        let inward = (ship.velocity - body_velocity).dot(normal);
        if inward < 0.0 {
            ship.velocity -= normal * inward;
        }
    }
}
