//! Ancestry presets for the SRD ancestries. Every sound pattern here is
//! original to Arda; none follows a published fictional language.

use super::{Spec, Stress, BASE};

/// Dwarvish: heavy stops, back vowels, closed syllables, r-clusters.
pub(crate) const DWARVISH: Spec = Spec {
    name: "dwarvish",
    consonants: "b4 d5 g4 k4 t2 x2 th2 z3 m4 n4 r6 l2 s1 v1",
    vowels: "a4 u5 o4 i2 e1",
    diphthongs: "a+i u+i",
    clusters: "b+r d+r g+r k+r t+r th+r",
    codas: "r5 n4 m4 d4 k3 g3 z3 th2 l2 x2 s1",
    coda_clusters: "r+n r+m r+d r+g r+k n+d l+d l+k r+z z+d m+b",
    initial_onset: 860,
    cluster_rate: 300,
    medial_coda: 800,
    final_coda: 950,
    diph_rate: 40,
    max_medial: 3,
    roots: [7, 3, 0],
    stress: Stress::Initial,
    head_first: 300,
    linker: 300,
    epenthetic: "u",
    ortho: "x=kh",
    fem: "i|a|u+n|i+s",
    masc: "||i+n|u+r|o+k",
    family: [3, 1, 0, 3, 3],
    byname: 300,
    ..BASE
};

/// Elvish: sibilants and glide clusters, vowel-rich, end-stressed.
pub(crate) const ELVISH: Spec = Spec {
    name: "elvish",
    consonants: "s5 sh2 v3 f2 th2 l5 n5 m2 r3 y3 w1 t3 k2 h1",
    vowels: "i5 e5 a4 o2 u1 i:1 e:1",
    diphthongs: "a+i e+i i+a i+e a+e",
    clusters: "s+y t+y l+y n+y",
    codas: "n5 s4 l4 r2 th1",
    initial_onset: 640,
    cluster_rate: 220,
    medial_coda: 300,
    final_coda: 450,
    diph_rate: 180,
    hiatus: true,
    roots: [2, 6, 2],
    stress: Stress::Final,
    head_first: 500,
    linker: 600,
    epenthetic: "i",
    fem: "a|e|i+a|e+s",
    masc: "|i+n|a+s|e+v",
    family: [1, 0, 2, 5, 0],
    byname: 80,
    ..BASE
};

/// Halfling: bouncy labials, doubled consonants, homely endings.
pub(crate) const HALFLING: Spec = Spec {
    name: "halfling",
    consonants: "b5 p4 t4 d4 k3 g3 m4 n4 l5 r3 w3 f2 s3 h2 v1",
    vowels: "o5 i4 a4 e3 u3",
    diphthongs: "o+u",
    clusters: "b+l p+l b+r t+r k+l f+l",
    codas: "n4 m3 l4 p3 b2 d3 k3 t3 s2 g2",
    coda_clusters: "m+p n+d l+d n+k",
    initial_onset: 900,
    cluster_rate: 120,
    medial_coda: 650,
    final_coda: 600,
    geminates: true,
    roots: [4, 6, 0],
    stress: Stress::Initial,
    head_first: 50,
    epenthetic: "e",
    final_spell: "i=y",
    fem: "a|i|e+l|o+s+a",
    masc: "o||i+n|e+r",
    family: [1, 3, 3, 3, 0],
    byname: 150,
    ..BASE
};

/// Gnomish: quick buzzing polysyllables with z and doubled consonants.
pub(crate) const GNOMISH: Spec = Spec {
    name: "gnomish",
    consonants: "b3 p3 t3 d3 k4 g3 z5 s2 f2 m3 n5 l4 r3 w1 sh1 v1",
    vowels: "i5 e4 a3 o4 u2 e:2 o:1",
    clusters: "b+l k+l g+l f+l p+l t+w",
    codas: "n5 k3 z4 b2 m2 l3 p2 t1 s2",
    coda_clusters: "n+k m+b n+z",
    initial_onset: 900,
    cluster_rate: 150,
    medial_coda: 500,
    final_coda: 500,
    diph_rate: 0,
    geminates: true,
    roots: [3, 5, 3],
    stress: Stress::Penult,
    head_first: 200,
    epenthetic: "i",
    fem: "a|i|i+n+a|e+t",
    masc: "o|i+k|e+n|u+s",
    family: [1, 3, 1, 4, 0],
    byname: 250,
    ..BASE
};

/// Orcish: gutturals, glottal breaks, closed back-vowelled syllables.
pub(crate) const ORCISH: Spec = Spec {
    name: "orcish",
    consonants: "g5 k4 q2 '2 d3 b3 z3 zh2 sh3 x4 gh3 r6 m2 n3 t3",
    vowels: "a5 u5 o3 i2",
    clusters: "g+r k+r b+r d+r",
    codas: "k5 g5 r5 z4 sh4 x3 zh2 t2 d3 m2 n3 q1",
    coda_clusters: "r+g r+k r+z r+sh n+k z+g",
    initial_onset: 900,
    cluster_rate: 250,
    medial_coda: 800,
    final_coda: 950,
    diph_rate: 0,
    max_medial: 3,
    roots: [7, 3, 0],
    stress: Stress::Initial,
    head_first: 300,
    linker: 0,
    epenthetic: "u",
    fem: "a|u+sh|a+x",
    masc: "||u+k|a+g",
    family: [1, 0, 0, 3, 5],
    byname: 350,
    ..BASE
};

/// Draconic: hissing, rolling r, x-codas, long vowels, end-stressed.
pub(crate) const DRACONIC: Spec = Spec {
    name: "draconic",
    consonants: "s4 z3 sh2 x2 th3 r6 v4 k4 t2 d2 y1 h2 n3 m2 l3",
    vowels: "a5 i4 e3 o3 u1 a:1",
    clusters: "v+r th+r k+r",
    codas: "x2 s4 r5 th3 n3 k2 sh1 z2 l2",
    coda_clusters: "r+x r+s",
    initial_onset: 900,
    cluster_rate: 150,
    medial_coda: 500,
    final_coda: 800,
    diph_rate: 0,
    roots: [2, 6, 2],
    stress: Stress::Final,
    head_first: 600,
    linker: 300,
    epenthetic: "a",
    ortho: "x=x",
    fem: "a|i+n|i+s+a",
    masc: "|a+x|o+r|i+s",
    family: [2, 0, 0, 2, 4],
    byname: 200,
    ..BASE
};

/// Infernal: dark sonorants, buzzing fricatives, glottal breaks.
pub(crate) const INFERNAL: Spec = Spec {
    name: "infernal",
    consonants: "b4 m4 z4 zh2 k3 x2 l4 r4 th3 n3 v2 g2 s2 '1",
    vowels: "a5 e3 o4 u3 i3 o:1",
    diphthongs: "a+e o+u",
    clusters: "b+r k+r th+r b+l g+l z+r",
    codas: "l4 r4 th4 z3 m3 n3 k3 x2 sh1",
    coda_clusters: "l+z r+m r+th l+th",
    initial_onset: 850,
    cluster_rate: 180,
    medial_coda: 600,
    final_coda: 700,
    diph_rate: 60,
    roots: [3, 6, 1],
    stress: Stress::Penult,
    head_first: 400,
    epenthetic: "o",
    ortho: "x=kh o:=oa",
    fem: "a|e+th|i+s",
    masc: "|o+s|u+l|a+x",
    family: [1, 0, 2, 3, 2],
    byname: 250,
    ..BASE
};
