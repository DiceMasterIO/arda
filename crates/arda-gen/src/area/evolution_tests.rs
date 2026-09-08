use super::*;

#[test]
fn nominal_area_cut_does_not_pin_or_taper_a_plateau() {
    // logic/02 shared terrain correction: 512-cell storage partitions are
    // not physical boundaries, including cells on both sides of the cut.
    let (width, height) = (1_030, 17);
    let mut terrain = vec![100_000; width * height];
    let coarse = terrain.clone();
    evolve(&mut terrain, &coarse, width, height).unwrap();
    let interior = terrain[8 * width + 400];
    assert!(interior > 100_000);
    for x in [510, 511, 512, 513, 600] {
        assert_eq!(terrain[8 * width + x], interior, "cut cell {x}");
    }
    assert_eq!(terrain[8 * width], 100_000);
    assert_eq!(terrain[8 * width + width - 1], 100_000);
}

#[test]
fn nominal_area_cut_does_not_change_uniform_plane_evolution() {
    let (width, height) = (1_030, 17);
    let mut terrain: Vec<_> = (0..height)
        .flat_map(|y| std::iter::repeat_n(500_000 - y as i32 * 1_000, width))
        .collect();
    let coarse = vec![100_000; terrain.len()];
    let before = terrain.clone();
    evolve(&mut terrain, &coarse, width, height).unwrap();
    let interior = terrain[8 * width + 400];
    assert_ne!(interior, before[8 * width + 400]);
    for x in [510, 511, 512, 513, 600] {
        assert_eq!(terrain[8 * width + x], interior, "cut cell {x}");
    }
}

#[test]
fn rectangular_evolution_is_deterministic_and_keeps_sea_and_outer_rim() {
    let (width, height) = (13, 9);
    let mut terrain: Vec<i32> = (0..width * height)
        .map(|i| 100_000 + ((i * 13 + i / width * 97) % 701) as i32 * 100)
        .collect();
    terrain[4 * width + 6] = -100_000;
    terrain[5 * width + 6] = 0;
    let before = terrain.clone();
    let coarse = vec![100_000; terrain.len()];
    let mut repeat = terrain.clone();
    evolve(&mut terrain, &coarse, width, height).unwrap();
    evolve(&mut repeat, &coarse, width, height).unwrap();
    assert_eq!(terrain, repeat);
    assert_ne!(terrain, before);
    for (i, &original) in before.iter().enumerate() {
        let (x, y) = (i % width, i / width);
        if original <= 0 || x == 0 || y == 0 || x + 1 == width || y + 1 == height {
            assert_eq!(terrain[i], original, "fixed cell {x},{y}");
        } else {
            assert!(terrain[i] > 0);
        }
    }
}

#[test]
fn invalid_rectangles_are_rejected_before_mutation() {
    let original = vec![100; 6];
    for (width, height) in [(1, 6), (3, 3), (usize::MAX, 2)] {
        let mut terrain = original.clone();
        assert!(evolve(&mut terrain, &original, width, height).is_err());
        assert_eq!(terrain, original);
    }
    let mut terrain = original.clone();
    assert!(evolve(&mut terrain, &[100; 5], 3, 2).is_err());
    assert_eq!(terrain, original);
}

#[test]
fn catchment_west_of_nominal_cut_increases_incision_east_of_it() {
    // Both worlds have identical terrain around the x=512 receiving cell.
    // Moving the far-upstream watershed must change its incision despite the
    // intervening publication cut; uplift and local creep stay identical.
    let (width, height) = (520, 7);
    let mut full: Vec<i32> = (0..width * height)
        .map(|i| 2_000_000 - (i % width) as i32 * 1_000 + ((i / width) as i32 - 3).abs() * 2_000)
        .collect();
    let mut diverted = full.clone();
    for y in 0..height {
        for x in 0..501 {
            diverted[y * width + x] = 500_000 + x as i32 * 1_000 + (y as i32 - 3).abs() * 2_000;
        }
    }
    let coarse = vec![0; full.len()];
    let at = 3 * width + 512;
    assert_eq!(full[at], diverted[at]);
    evolve_steps(&mut full, &coarse, width, height, 1).unwrap();
    evolve_steps(&mut diverted, &coarse, width, height, 1).unwrap();
    assert!(
        full[at] < diverted[at],
        "upstream catchment must deepen the shared channel: {} versus {}",
        full[at],
        diverted[at]
    );
}

#[test]
fn draining_valley_does_not_gain_a_pit_against_a_rising_receiver() {
    // logic/02 shared terrain correction: the explicit old-receiver cap
    // incised x=512 by 1000 mm, leaving it at 100600 mm, while its receiver
    // rose to 100628 mm through creep. This was a new closed depression in
    // a previously draining valley; a full downstream-first solve must not
    // create that 28 mm pit, even for a large contributing catchment.
    let (width, height) = (520, 65);
    let mut terrain: Vec<i32> = (0..width * height)
        .map(|i| {
            let (x, y) = ((i % width) as i32, (i / width) as i32);
            let channel = if x <= 513 {
                100_000 + (513 - x) * 1_000
            } else {
                100_000 - (x - 513) * 50
            };
            channel + (y - 32).abs() * 10_000
        })
        .collect();
    let coarse = vec![0; terrain.len()];
    let at = 32 * width + 512;
    assert!(terrain[at] > terrain[at + 1]);
    evolve_steps(&mut terrain, &coarse, width, height, 1).unwrap();
    assert!(
        terrain[at] > terrain[at + 1],
        "incision inverted the draining valley: {} <= {}",
        terrain[at],
        terrain[at + 1]
    );
}

#[test]
fn physical_bowl_is_submerged_but_routing_epsilon_is_not_a_lake() {
    let grid = Grid::checked(7, 5).unwrap();
    let mut terrain = vec![100_000; grid.count];
    let mut scratch = Scratch::new(grid).unwrap();
    let at = 2 * grid.width + 3;
    scratch.flood(&terrain, grid, 0);
    assert_eq!(scratch.routing[at], 100_000);
    scratch.flood(&terrain, grid, 1);
    assert!(scratch.routing[at] > 100_000);

    terrain[at] = 60_000;
    terrain[at + 1] = 50_000;
    let original = terrain.clone();
    scratch.flood(&terrain, grid, 0);
    assert_eq!(scratch.routing[at], 100_000);
    assert_eq!(scratch.routing[at + 1], 100_000);
    assert_eq!(terrain, original);
    let routing: Vec<_> = terrain.iter().map(|&h| i64::from(h)).collect();
    assert_eq!(
        physical_incision(
            &terrain,
            &terrain,
            &routing,
            100_000,
            mfd::ONE * 100,
            grid,
            at
        ),
        0
    );
    assert!(
        physical_incision(
            &terrain,
            &terrain,
            &routing,
            60_000,
            mfd::ONE * 100,
            grid,
            at
        ) > 0
    );
    evolve_steps(
        &mut terrain,
        &vec![0; grid.count],
        grid.width,
        grid.height,
        1,
    )
    .unwrap();
    assert!(
        terrain[at] > original[at],
        "creep still smooths a submerged bed"
    );
    assert!(
        terrain[at] < 100_000,
        "the physical bowl remains a depression"
    );
}

#[test]
fn implicit_stream_power_matches_the_linear_physical_response() {
    // Exact backward-Euler n=1 responses with the existing coefficient,
    // including a fractional catchment and the diagonal physical run.
    assert_eq!(implicit_incision(mfd::ONE * 100, 11_000, false), 1_000);
    assert_eq!(implicit_incision(mfd::ONE * 10_000, 9_000, false), 4_500);
    assert_eq!(
        implicit_incision(mfd::ONE * 1_000_000, 11_000, false),
        10_000
    );
    assert_eq!(implicit_incision(mfd::ONE * 100, 15_140, true), 1_000);
    assert_eq!(implicit_incision(mfd::ONE * 9 / 4, 20_300, false), 300);
    assert_eq!(implicit_incision(mfd::ONE * 100, 50, false), 4);
    assert_eq!(implicit_incision(mfd::ONE * 100, 0, false), 0);
    assert_eq!(implicit_incision(mfd::ONE * 100, -100, false), 0);
    for area in [mfd::ONE, mfd::ONE * 100, u64::MAX] {
        for drop in [1, 2, 50, 100, 10_000, i64::from(u32::MAX)] {
            let removed = implicit_incision(area, drop, false);
            assert!(removed >= 0 && removed < drop);
        }
    }
}

#[test]
fn broad_physical_bowl_survives_the_full_evolution() {
    let grid = Grid::checked(41, 31).unwrap();
    let mut terrain = vec![100_000; grid.count];
    for y in 8..23 {
        for x in 12..29 {
            terrain[y * grid.width + x] = 30_000;
        }
    }
    let center = 15 * grid.width + 20;
    evolve(&mut terrain, &vec![0; grid.count], grid.width, grid.height).unwrap();
    let mut scratch = Scratch::new(grid).unwrap();
    scratch.flood(&terrain, grid, 0);
    assert!(
        terrain[center] >= 30_000,
        "submerged floor must not be incised"
    );
    assert!(
        terrain[center] < 100_000,
        "the broad physical bowl survives"
    );
    assert_eq!(scratch.routing[center], 100_000);
    assert!(terrain.iter().all(|&h| h > 0));
}

#[test]
fn rectangular_helpers_preserve_existing_square_physics() {
    let grid = Grid::checked(7, 7).unwrap();
    let terrain: Vec<i32> = (0..grid.count)
        .map(|i| 100_000 + ((i * 13 + i / grid.width * 97) % 701) as i32 * 100)
        .collect();
    let mut scratch = Scratch::new(grid).unwrap();
    scratch.flood(&terrain, grid, 0);
    let physical: Vec<i32> = scratch.routing.iter().map(|&v| v as i32).collect();
    assert_eq!(physical, super::super::physical_spill::surface(&terrain, 7));
    scratch.flood(&terrain, grid, 1);
    let routing: Vec<i32> = scratch.routing.iter().map(|&v| v as i32).collect();
    mfd::accumulate_rectangular(
        &scratch.routing,
        grid.width,
        grid.height,
        &scratch.order,
        &mut scratch.area,
    )
    .unwrap();
    assert_eq!(scratch.area, mfd::accumulate_grid(&routing, 7));
}

#[test]
fn full_integer_height_range_keeps_checked_routing_and_sea() {
    let grid = Grid::checked(7, 5).unwrap();
    let mut terrain = vec![i32::MAX; grid.count];
    let mut scratch = Scratch::new(grid).unwrap();
    scratch.flood(&terrain, grid, 1);
    assert!(scratch.routing[2 * grid.width + 3] > i64::from(i32::MAX));
    let coarse = vec![i32::MAX; grid.count];
    evolve_steps(&mut terrain, &coarse, grid.width, grid.height, 1).unwrap();
    assert!(terrain.iter().all(|&h| h == i32::MAX));
    terrain[2 * grid.width + 3] = i32::MIN;
    evolve_steps(&mut terrain, &coarse, grid.width, grid.height, 1).unwrap();
    assert_eq!(terrain[2 * grid.width + 3], i32::MIN);
    assert!(terrain
        .iter()
        .enumerate()
        .all(|(i, &h)| i == 2 * grid.width + 3 || h > 0));
}

#[test]
fn scratch_budget_covers_all_reserved_buffers_without_growth() {
    let grid = Grid::checked(13, 9).unwrap();
    let mut scratch = Scratch::new(grid).unwrap();
    let capacities = |s: &Scratch| {
        s.next.capacity() * size_of::<i32>()
            + s.physical.capacity() * size_of::<i32>()
            + s.routing.capacity() * size_of::<i64>()
            + s.order.capacity() * size_of::<u32>()
            + s.area.capacity() * size_of::<u64>()
            + s.seen.capacity() * size_of::<u8>()
            + s.queue.capacity() * size_of::<QueueEntry>()
            + size_of::<Scratch>()
    };
    let budget = scratch_bytes(grid.width, grid.height).unwrap();
    assert_eq!(capacities(&scratch) as u128, budget);
    for height in [i32::MIN, -1, 0, 100, i32::MAX] {
        let terrain = vec![height; grid.count];
        scratch.flood(&terrain, grid, 0);
        scratch.flood(&terrain, grid, 1);
        assert_eq!(capacities(&scratch) as u128, budget);
    }
    assert!(scratch_bytes(usize::MAX, 2).is_err());
    if usize::BITS > 32 {
        assert!(scratch_bytes(65_536, 65_536).is_err());
    }
}
