use super::*;

fn rim(width: usize, height: usize) -> Vec<u8> {
    (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            u8::from(x == 0 || y == 0 || x == width - 1 || y == height - 1)
        })
        .collect()
}

#[test]
fn fill_removes_every_closed_pit() {
    let (w, h) = (40, 30);
    let mut z: Vec<i32> = (0..w * h)
        .map(|i| i32::try_from(hash_2d(3, (i % w) as i32, (i / w) as i32) % 5000).unwrap())
        .collect();
    let flags = rim(w, h);
    let mut next = vec![0; w * h];
    let mut closed = vec![0; w * h];
    fill(&mut z, w, h, &flags, 1, &mut next, &mut closed).unwrap();
    for i in 0..w * h {
        if flags[i] & FIXED != 0 {
            continue;
        }
        let lower = (0..8).any(|k| neighbour(i, w, h, k).is_some_and(|n| z[n] < z[i]));
        assert!(lower, "cell {i} remains a pit");
    }
}

#[test]
fn local_fill_removes_every_closed_pit_like_the_global_fill() {
    let (w, h) = (60, 45);
    let mut z: Vec<i32> = (0..w * h)
        .map(|i| {
            let (x, y) = ((i % w) as i32, (i / w) as i32);
            x * 400 + y * 150 + i32::try_from(hash_2d(4, x, y) % 1500).unwrap()
        })
        .collect();
    let flags = rim(w, h);
    let mut stamp = vec![0; w * h];
    let mut closed = vec![0; w * h];
    let before = z.clone();
    fill_local(&mut z, w, h, &flags, 1, (&mut stamp, &mut closed), true).unwrap();
    assert!(pits(&z, w, h, &flags).is_empty());
    assert!(
        z.iter().zip(&before).all(|(a, b)| a >= b),
        "fill only raises"
    );
}

#[test]
fn enclosed_below_sea_pockets_are_filled_not_kept_as_sea() {
    // Sea on the west third; an enclosed -2 m pocket inside the land.
    let (w, h) = (30, 20);
    let mut z: Vec<i32> = (0..w * h)
        .map(|i| {
            if i % w < 10 {
                -5_000
            } else {
                3_000 + (i % w) as i32 * 100
            }
        })
        .collect();
    z[10 * w + 20] = -2_000;
    let mut flags = vec![0; w * h];
    open_sea_flags(&z, w, h, &mut flags);
    assert_eq!(flags[10 * w + 5] & FIXED, FIXED, "open sea is fixed");
    assert_eq!(flags[10 * w + 20] & FIXED, 0, "the pocket is not sea");
    let mut next = vec![0; w * h];
    let mut closed = vec![0; w * h];
    fill(&mut z, w, h, &flags, 1, &mut next, &mut closed).unwrap();
    assert!(z[10 * w + 20] > 0, "the pocket is lifted to its spill");
}

#[test]
fn order_is_upstream_first_and_area_conserves_cells() {
    let (w, h) = (25, 19);
    let mut z: Vec<i32> = (0..w * h)
        .map(|i| i32::try_from(hash_2d(9, (i % w) as i32, (i / w) as i32) % 3000).unwrap())
        .collect();
    let flags = rim(w, h);
    let mut next = vec![0; w * h];
    let mut scratch = vec![0; w * h];
    fill(&mut z, w, h, &flags, 1, &mut next, &mut scratch).unwrap();
    let mut rcv = vec![0; w * h];
    receivers(&z, w, h, &flags, 5, 40_000, &mut rcv);
    let mut order = vec![0; w * h];
    upstream_order(&rcv, w, h, &mut scratch, &mut order);
    let mut pos = vec![0; w * h];
    for (p, &i) in order.iter().enumerate() {
        pos[i as usize] = p;
    }
    for i in 0..w * h {
        let r = receiver_index(i, w, h, rcv[i]);
        if r != i {
            assert!(pos[i] < pos[r]);
            assert!(z[r] < z[i], "receiver must be lower");
        }
    }
    let weight = vec![1_u16; w * h];
    let mut area = vec![0; w * h];
    accumulate(&rcv, w, h, &order, 1, Some(&weight), &mut area);
    let outlets: u64 = (0..w * h)
        .filter(|&i| receiver_index(i, w, h, rcv[i]) == i)
        .map(|i| area[i])
        .sum();
    assert_eq!(outlets, (w * h) as u64);
}
