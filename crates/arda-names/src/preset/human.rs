//! Human culture presets, keyed like `arda-settle`'s culture regions.
//! All inventories and weights are original to Arda.

use super::{Spec, Stress, BASE};

/// Mellow lowland: soft stops, liquids, closed first syllables.
pub(crate) const HEARTLAND: Spec = Spec {
    name: "heartland",
    consonants: "p2 b3 t4 d4 k3 g2 f2 v1 th1 s4 h2 m3 n5 l5 r5 w3 y1",
    vowels: "a5 e4 i3 o4 u2 e:1",
    diphthongs: "a+i o+u e+a",
    clusters: "b+r d+r t+r k+r g+r f+r b+l k+l f+l s+t s+k s+w th+r g+l s+l s+n",
    codas: "n5 r4 l4 d3 t2 k2 m2 s3 th1 f1 ng1",
    coda_clusters: "n+d r+d l+d r+n l+m n+t r+k l+k s+t l+f",
    initial_onset: 820,
    cluster_rate: 160,
    medial_coda: 500,
    final_coda: 700,
    roots: [6, 4, 0],
    stress: Stress::Initial,
    head_first: 40,
    epenthetic: "e",
    ortho: "x=gh e:=ee",
    final_spell: "i=y",
    fem: "a|@+th|i+n|e+l+a",
    masc: "||o|i+k|e+r",
    family: [3, 4, 2, 1, 0],
    byname: 150,
    ..BASE
};

/// Clipped highland: short closed syllables, velar fricatives.
pub(crate) const HIGHLAND: Spec = Spec {
    name: "highland",
    consonants: "b3 d3 g3 k4 t3 m3 n4 l4 r5 s2 sh1 x3 v1 f1 ch1",
    vowels: "a5 o3 u4 i3 e2 ue1",
    diphthongs: "a+i o+i",
    clusters: "d+r b+r g+r k+r t+r g+l b+l k+l s+t s+k",
    codas: "n5 r5 l4 k4 g3 d3 m3 x3 s2 t2 ch1",
    coda_clusters: "r+n r+k l+k n+d r+d l+d r+x n+k l+g r+g",
    initial_onset: 930,
    cluster_rate: 250,
    medial_coda: 700,
    final_coda: 950,
    diph_rate: 50,
    roots: [9, 2, 0],
    stress: Stress::Initial,
    head_first: 700,
    linker: 150,
    epenthetic: "a",
    ortho: "k=c x=ch ue=y",
    k_front: "k",
    fem: "a|i+n|a+g|e+t",
    masc: "|||a+n|o+k",
    family: [2, 1, 1, 0, 5],
    byname: 250,
    ..BASE
};

/// Soft forest folk: fricatives, liquids, diphthongs, penultimate stress.
pub(crate) const SYLVAN: Spec = Spec {
    name: "sylvan",
    consonants: "th3 f2 v3 s3 h2 l6 r4 n5 m3 w3 y1 t2 d2 k1 g1",
    vowels: "a4 e5 i4 o2 u1 e:1 i:1",
    diphthongs: "a+e e+i a+i",
    clusters: "th+r f+r f+l s+l s+w th+w t+w g+w",
    codas: "n5 l5 r3 th3 s2 f1 m1",
    coda_clusters: "l+f l+d n+d r+n l+th",
    initial_onset: 780,
    cluster_rate: 140,
    medial_coda: 350,
    final_coda: 600,
    diph_rate: 160,
    roots: [4, 6, 0],
    stress: Stress::Penult,
    head_first: 400,
    linker: 500,
    epenthetic: "e",
    ortho: "e:=ea i:=ee",
    fem: "a|ue+n|e+th|i+a",
    masc: "|a+s|o+n|i+r",
    family: [1, 1, 2, 4, 0],
    byname: 100,
    ..BASE
};

/// Harsh northern seafarers: s-clusters, long vowels, patronymics.
pub(crate) const COASTAL: Spec = Spec {
    name: "coastal",
    consonants: "k5 t4 g3 d2 b2 p1 f2 v3 s4 h4 m2 n4 l4 r5 y2 th1",
    vowels: "a4 e3 i3 o3 u3 ue1 a:1",
    diphthongs: "e+i a+u",
    clusters: "s+k s+t s+n s+v h+v h+r h+l k+r g+r t+r b+r f+r f+l k+v th+r",
    codas: "r5 n4 l4 k4 g3 d3 t3 s3 m2 f1 th1",
    coda_clusters: "r+k l+k n+d r+d l+d r+n l+f n+g r+g l+t s+k",
    initial_onset: 860,
    cluster_rate: 300,
    medial_coda: 600,
    final_coda: 800,
    diph_rate: 80,
    roots: [6, 4, 0],
    stress: Stress::Initial,
    head_first: 40,
    epenthetic: "e",
    ortho: "y=j o:=oe ue=y",
    fem: "a|e|i+d|u+n",
    masc: "||i|a+r|e+n",
    family: [6, 1, 1, 1, 0],
    byname: 250,
    ..BASE
};

/// Flowing south: open syllables, liquids, penultimate stress, head-first.
pub(crate) const SOUTHERN: Spec = Spec {
    name: "southern",
    consonants: "p2 b2 t4 d3 k3 g2 f1 v3 s5 z1 m4 n5 l5 r6 ny1 y1",
    vowels: "a6 e4 i4 o5 u2",
    diphthongs: "a+i e+i i+a i+o",
    clusters: "t+r d+r p+r b+r k+r g+r f+r p+l b+l k+l f+l",
    codas: "n5 r4 l4 s4 m1",
    initial_onset: 880,
    cluster_rate: 140,
    medial_coda: 280,
    final_coda: 220,
    diph_rate: 100,
    hiatus: true,
    roots: [1, 7, 2],
    stress: Stress::Penult,
    head_first: 850,
    linker: 500,
    epenthetic: "a",
    ortho: "k=c ny=gn y=i",
    k_front: "ch",
    fem: "a|e+l+a|i+n+a|i+a",
    masc: "o|i+o|e+n|a+n+o",
    family: [2, 2, 4, 1, 0],
    byname: 120,
    ..BASE
};

/// Dry marchland steppe folk: vowel harmony, final stress, no clusters.
pub(crate) const BORDERLAND: Spec = Spec {
    name: "borderland",
    consonants: "b3 t4 d3 k4 q3 g2 s4 sh3 z2 ch3 dj2 m3 n5 l3 r4 y1 x2",
    vowels: "a5 e4 i3 o3 u4 ue2",
    codas: "n5 r5 l3 s3 sh2 k3 q2 t3 z2 m2 y2",
    coda_clusters: "r+t l+t n+k r+k y+n",
    initial_onset: 720,
    cluster_rate: 0,
    medial_coda: 700,
    final_coda: 750,
    diph_rate: 0,
    harmony: true,
    roots: [4, 6, 0],
    stress: Stress::Final,
    head_first: 0,
    epenthetic: "i",
    ortho: "ue=y",
    fem: "a+y|a|i+n|e+k",
    masc: "|a+n|u+r|e+k",
    family: [4, 1, 1, 0, 3],
    byname: 250,
    ..BASE
};

/// The substrate: plain archaic CV(C) with no clusters, borrowed into every
/// language as river and mountain names.
pub(crate) const ANCIENT: Spec = Spec {
    name: "ancient",
    consonants: "p2 t4 k3 b2 d3 g2 s4 m4 n5 l4 r4 w3 y2 h1",
    vowels: "a5 e3 i4 o3 u4",
    codas: "n4 r3 l3 s3 m2",
    cluster_rate: 0,
    medial_coda: 300,
    final_coda: 450,
    roots: [3, 5, 0],
    stress: Stress::Initial,
    drop: 0,
    ..BASE
};
