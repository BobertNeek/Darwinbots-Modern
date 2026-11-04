#[derive(Clone, Debug)]
pub struct SpatialIndex {
    cell_size: f32,
    positions: Vec<[f32; 2]>,
    slots: Vec<usize>,
    origin: (i32, i32),
    dimensions: (usize, usize),
    cells: Vec<Vec<usize>>,
}

impl Default for SpatialIndex {
    fn default() -> Self {
        Self {
            cell_size: 1.0,
            positions: Vec::new(),
            slots: Vec::new(),
            origin: (0, 0),
            dimensions: (0, 0),
            cells: Vec::new(),
        }
    }
}

impl SpatialIndex {
    pub fn build(positions: &[[f32; 2]], cell_size: f32) -> Self {
        Self::build_with_slots(positions.iter().copied().enumerate(), cell_size)
    }

    pub(crate) fn build_with_slots(
        entries: impl IntoIterator<Item = (usize, [f32; 2])>,
        cell_size: f32,
    ) -> Self {
        let cell_size = cell_size.max(1.0);
        let entries: Vec<_> = entries.into_iter().collect();
        if entries.is_empty() {
            return Self {
                cell_size,
                ..Self::default()
            };
        }
        let coordinates: Vec<_> = entries
            .iter()
            .map(|(_, position)| cell(*position, cell_size))
            .collect();
        let min_x = coordinates.iter().map(|value| value.0).min().unwrap();
        let max_x = coordinates.iter().map(|value| value.0).max().unwrap();
        let min_y = coordinates.iter().map(|value| value.1).min().unwrap();
        let max_y = coordinates.iter().map(|value| value.1).max().unwrap();
        let dimensions = ((max_x - min_x + 1) as usize, (max_y - min_y + 1) as usize);
        let mut index = Self {
            cell_size,
            positions: Vec::with_capacity(entries.len()),
            slots: Vec::with_capacity(entries.len()),
            origin: (min_x, min_y),
            dimensions,
            cells: vec![Vec::new(); dimensions.0 * dimensions.1],
        };
        for ((slot, position), coordinate) in entries.into_iter().zip(coordinates) {
            let entry = index.positions.len();
            index.positions.push(position);
            index.slots.push(slot);
            let dense = index.dense_index(coordinate).unwrap();
            index.cells[dense].push(entry);
        }
        index
    }

    pub(crate) fn rebuild_from_soa(
        &mut self,
        positions: &[[f32; 2]],
        alive: &[bool],
        cell_size: f32,
    ) {
        let cell_size = cell_size.max(1.0);
        let mut bounds: Option<(i32, i32, i32, i32)> = None;
        for (slot, position) in positions.iter().enumerate() {
            if !alive.get(slot).copied().unwrap_or(false) {
                continue;
            }
            let coordinate = cell(*position, cell_size);
            bounds = Some(match bounds {
                None => (coordinate.0, coordinate.0, coordinate.1, coordinate.1),
                Some((min_x, max_x, min_y, max_y)) => (
                    min_x.min(coordinate.0),
                    max_x.max(coordinate.0),
                    min_y.min(coordinate.1),
                    max_y.max(coordinate.1),
                ),
            });
        }
        let Some((min_x, max_x, min_y, max_y)) = bounds else {
            self.positions.clear();
            self.slots.clear();
            self.cells.clear();
            self.dimensions = (0, 0);
            self.origin = (0, 0);
            self.cell_size = cell_size;
            return;
        };
        let dimensions = ((max_x - min_x + 1) as usize, (max_y - min_y + 1) as usize);
        if self.dimensions == dimensions && self.origin == (min_x, min_y) {
            for entries in &mut self.cells {
                entries.clear();
            }
        } else {
            self.cells = vec![Vec::new(); dimensions.0 * dimensions.1];
            self.dimensions = dimensions;
            self.origin = (min_x, min_y);
        }
        self.cell_size = cell_size;
        self.positions.clear();
        self.slots.clear();
        self.positions
            .reserve(positions.len().saturating_sub(self.positions.capacity()));
        self.slots
            .reserve(positions.len().saturating_sub(self.slots.capacity()));
        for (slot, position) in positions.iter().copied().enumerate() {
            if !alive.get(slot).copied().unwrap_or(false) {
                continue;
            }
            let entry = self.positions.len();
            self.positions.push(position);
            self.slots.push(slot);
            let dense = self.dense_index(cell(position, cell_size)).unwrap();
            self.cells[dense].push(entry);
        }
    }

    pub fn neighbors(&self, position: [f32; 2], radius: f32) -> Vec<usize> {
        let radius = radius.max(0.0);
        let min = cell([position[0] - radius, position[1] - radius], self.cell_size);
        let max = cell([position[0] + radius, position[1] + radius], self.cell_size);
        let Some((min, max)) = self.clipped_cells(min, max) else {
            return Vec::new();
        };
        let radius_squared = radius * radius;
        let mut neighbors = Vec::new();
        for y in min.1..=max.1 {
            for x in min.0..=max.0 {
                let Some(entries) = self.cell_entries((x, y)) else {
                    continue;
                };
                for entry in entries {
                    let candidate = self.positions[*entry];
                    let dx = candidate[0] - position[0];
                    let dy = candidate[1] - position[1];
                    if dx * dx + dy * dy <= radius_squared {
                        neighbors.push(self.slots[*entry]);
                    }
                }
            }
        }
        neighbors.sort_unstable();
        neighbors
    }

    pub fn nearest(
        &self,
        position: [f32; 2],
        exclude: Option<usize>,
        max_radius: f32,
    ) -> Option<usize> {
        let (grid_min, grid_max) = self.occupied_bounds()?;
        let cell_center = cell(position, self.cell_size);
        let center = [i64::from(cell_center.0), i64::from(cell_center.1)];
        let requested_ring = i64::from((max_radius.max(0.0) / self.cell_size).ceil() as i32);
        let first_ring = (0..2)
            .map(|axis| {
                (grid_min[axis] - center[axis])
                    .max(center[axis] - grid_max[axis])
                    .max(0)
            })
            .max()
            .unwrap();
        let max_ring = requested_ring.min(
            (0..2)
                .map(|axis| {
                    (grid_min[axis] - center[axis])
                        .abs()
                        .max((grid_max[axis] - center[axis]).abs())
                })
                .max()
                .unwrap(),
        );
        let max_distance_squared = max_radius * max_radius;
        let mut best: Option<(usize, f32)> = None;
        for ring in first_ring..=max_ring {
            let left = center[0] - ring;
            let right = center[0] + ring;
            let top = center[1] - ring;
            let bottom = center[1] + ring;
            for y in top.max(grid_min[1])..=bottom.min(grid_max[1]) {
                for x in left.max(grid_min[0])..=right.min(grid_max[0]) {
                    if ring > 0 && x != left && x != right && y != top && y != bottom {
                        continue;
                    }
                    let Some(entries) = self.cell_entries((x as i32, y as i32)) else {
                        continue;
                    };
                    for entry in entries {
                        let slot = self.slots[*entry];
                        if exclude == Some(slot) {
                            continue;
                        }
                        let candidate = self.positions[*entry];
                        let dx = candidate[0] - position[0];
                        let dy = candidate[1] - position[1];
                        let distance = dx * dx + dy * dy;
                        if distance > max_distance_squared {
                            continue;
                        }
                        if best.is_none_or(|(best_slot, best_distance)| {
                            distance < best_distance
                                || (distance == best_distance && slot < best_slot)
                        }) {
                            best = Some((slot, distance));
                        }
                    }
                }
            }
            if ring > 0
                && best.is_some_and(|(_, distance)| {
                    let unvisited_distance = ring as f32 * self.cell_size;
                    distance <= unvisited_distance * unvisited_distance
                })
            {
                break;
            }
        }
        best.map(|(slot, _)| slot)
    }

    pub fn segment_candidates(&self, start: [f32; 2], end: [f32; 2], padding: f32) -> Vec<usize> {
        let padding = padding.max(0.0);
        let minimum = [
            start[0].min(end[0]) - padding,
            start[1].min(end[1]) - padding,
        ];
        let maximum = [
            start[0].max(end[0]) + padding,
            start[1].max(end[1]) + padding,
        ];
        let min_cell = cell(minimum, self.cell_size);
        let max_cell = cell(maximum, self.cell_size);
        let Some((min_cell, max_cell)) = self.clipped_cells(min_cell, max_cell) else {
            return Vec::new();
        };
        let mut candidates = Vec::new();
        for y in min_cell.1..=max_cell.1 {
            for x in min_cell.0..=max_cell.0 {
                let Some(entries) = self.cell_entries((x, y)) else {
                    continue;
                };
                candidates.extend(entries.iter().map(|entry| self.slots[*entry]));
            }
        }
        candidates.sort_unstable();
        candidates.dedup();
        candidates
    }

    fn occupied_bounds(&self) -> Option<([i64; 2], [i64; 2])> {
        if self.dimensions.0 == 0 || self.dimensions.1 == 0 {
            return None;
        }
        let minimum = [i64::from(self.origin.0), i64::from(self.origin.1)];
        Some((
            minimum,
            [
                minimum[0] + self.dimensions.0 as i64 - 1,
                minimum[1] + self.dimensions.1 as i64 - 1,
            ],
        ))
    }

    fn clipped_cells(&self, min: (i32, i32), max: (i32, i32)) -> Option<((i32, i32), (i32, i32))> {
        let (grid_min, grid_max) = self.occupied_bounds()?;
        let low = [
            i64::from(min.0).max(grid_min[0]),
            i64::from(min.1).max(grid_min[1]),
        ];
        let high = [
            i64::from(max.0).min(grid_max[0]),
            i64::from(max.1).min(grid_max[1]),
        ];
        (low[0] <= high[0] && low[1] <= high[1]).then_some((
            (low[0] as i32, low[1] as i32),
            (high[0] as i32, high[1] as i32),
        ))
    }

    fn dense_index(&self, coordinate: (i32, i32)) -> Option<usize> {
        let x = coordinate.0 - self.origin.0;
        let y = coordinate.1 - self.origin.1;
        if x < 0 || y < 0 || x as usize >= self.dimensions.0 || y as usize >= self.dimensions.1 {
            return None;
        }
        Some(y as usize * self.dimensions.0 + x as usize)
    }

    fn cell_entries(&self, coordinate: (i32, i32)) -> Option<&[usize]> {
        self.dense_index(coordinate)
            .map(|index| self.cells[index].as_slice())
    }
}

fn cell(position: [f32; 2], cell_size: f32) -> (i32, i32) {
    (
        (position[0] / cell_size).floor() as i32,
        (position[1] / cell_size).floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const POINTS: [(usize, [f32; 2]); 5] = [
        (9, [-200.0, -100.0]),
        (7, [0.0, 0.0]),
        (1, [99.0, 100.0]),
        (3, [100.0, 101.0]),
        (5, [400.0, 500.0]),
    ];

    #[test]
    fn extreme_queries_on_empty_and_populated_grids_are_bounded() {
        let empty = SpatialIndex::default();
        assert!(empty.neighbors([0.0; 2], f32::MAX).is_empty());
        assert_eq!(empty.nearest([0.0; 2], None, f32::MAX), None);
        assert!(
            empty
                .segment_candidates([-f32::MAX; 2], [f32::MAX; 2], f32::MAX)
                .is_empty()
        );
        let grid = SpatialIndex::build_with_slots(POINTS, 100.0);
        assert_eq!(grid.neighbors([0.0; 2], f32::MAX), [1, 3, 5, 7, 9]);
        assert_eq!(grid.nearest([0.0; 2], None, f32::MAX), Some(7));
        assert_eq!(
            grid.segment_candidates([-f32::MAX; 2], [f32::MAX; 2], f32::MAX),
            [1, 3, 5, 7, 9]
        );
    }

    #[test]
    fn bounded_neighbor_and_nearest_queries_match_exhaustive_distances() {
        let grid = SpatialIndex::build_with_slots(POINTS, 100.0);
        for position in [[-500.0, 10.0], [0.0; 2], [100.0; 2], [1e9, -1e9]] {
            for radius in [0.0, 50.0, 100.0, 400.0, 2000.0, f32::MAX] {
                let mut expected: Vec<_> = POINTS
                    .into_iter()
                    .filter_map(|(slot, point)| {
                        let dx = point[0] - position[0];
                        let dy = point[1] - position[1];
                        let distance = dx * dx + dy * dy;
                        (distance <= radius * radius).then_some((slot, distance))
                    })
                    .collect();
                expected.sort_by_key(|&(slot, _)| slot);
                assert_eq!(
                    grid.neighbors(position, radius),
                    expected.iter().map(|&(slot, _)| slot).collect::<Vec<_>>()
                );
                for exclude in [None, Some(7)] {
                    let nearest = expected
                        .iter()
                        .filter(|&&(slot, _)| Some(slot) != exclude)
                        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                        .map(|&(slot, _)| slot);
                    assert_eq!(grid.nearest(position, exclude, radius), nearest);
                }
            }
        }
    }

    #[test]
    fn clipped_segment_queries_keep_the_same_conservative_cell_candidates() {
        let grid = SpatialIndex::build_with_slots(POINTS, 100.0);
        for (start, end, padding) in [
            ([-150.0; 2], [100.0; 2], 10.0),
            ([10000.0; 2], [20000.0; 2], 0.0),
            ([-f32::MAX; 2], [f32::MAX; 2], f32::MAX),
        ] {
            let min = cell(
                [
                    start[0].min(end[0]) - padding,
                    start[1].min(end[1]) - padding,
                ],
                100.0,
            );
            let max = cell(
                [
                    start[0].max(end[0]) + padding,
                    start[1].max(end[1]) + padding,
                ],
                100.0,
            );
            let mut expected: Vec<_> = POINTS
                .into_iter()
                .filter_map(|(slot, point)| {
                    let coordinate = cell(point, 100.0);
                    ((min.0..=max.0).contains(&coordinate.0)
                        && (min.1..=max.1).contains(&coordinate.1))
                    .then_some(slot)
                })
                .collect();
            expected.sort_unstable();
            assert_eq!(grid.segment_candidates(start, end, padding), expected);
        }
    }
}
