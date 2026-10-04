#!/usr/bin/env node
// Builds prompts.json from checklist.csv and the descriptions below.
//
//   node tools/art-gen/build-prompts.mjs [--check]
//
// prompts.json is the file generate.mjs reads; this script is how it is maintained. Edit a
// description here, run it, and commit both. `--check` exits non-zero when prompts.json is
// out of date instead of writing it.
//
// Slopify fills a Library image prompt's `{{keywords}}` from per-run values, and admission
// caps every keyword value at 200 characters on one line. So the long, shared part of each
// prompt (the style and the class rules) lives in a Library prompt per class, and each asset
// carries only short keywords: Subject, Shape and Detail, plus Variant for ground and water.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { entries as expansion, groundTargets, newKits, wallVariants } from "./expansion.mjs";
import { pair, readChecklist, render, slotsOf } from "./lib.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const checklistPath = join(here, "checklist.csv");
const outPath = join(here, "prompts.json");
// The placeholder library's records carry the 225's catalogue metadata (tags, blocking, cover),
// and its vocabulary is the one the importer writes, so every tag here must come from it.
const placeholderPath = join(here, "../../assets/tactical/placeholder/catalog.json");
const placeholder = JSON.parse(readFileSync(placeholderPath, "utf8"));
const valueMax = 200;

// ---------------------------------------------------------------------------------------------
// Style and class templates.

const style =
  "Top-down battle-map asset for a tabletop fantasy grid map, in a rich hand-painted style: " +
  "confident painterly brushwork, fine dark ink outlines, crisp small detail and a muted, " +
  "earthy, slightly warm palette. Strictly orthographic view from directly overhead: no " +
  "perspective, no horizon, no tilted or isometric angle. Soft light from the top-left, so " +
  "upper-left edges catch highlights and lower-right edges fall into gentle form shading; no " +
  "cast shadow and no drop shadow anywhere. No text, no letters, no numbers, no grid lines, " +
  "no frame, no border, no vignette, no watermark, no signature.";

const cutoutRules =
  "Exactly one subject, centred, filling about 70% of the frame's shorter side, on a fully " +
  "transparent background (a PNG with a real alpha channel, background: transparent; no white " +
  "or colour behind the subject); nothing else in the image: no floor, no ground patch, no base " +
  "or rug under it, no other objects, no shadow.";

const classes = {
  ground: {
    template: "arda-ground",
    textured: true,
    rules:
      "Seamless tileable texture, even lighting, no large light or dark patches, no single " +
      "dominant feature and no objects lying on it; fills the whole frame edge to edge, with no " +
      "background, border or empty margin anywhere.",
    subject: "Subject: a ground texture of {{Subject}}.",
  },
  water: {
    template: "arda-water",
    textured: true,
    rules:
      "Seamless tileable texture, even lighting, no large light or dark patches, the same depth " +
      "everywhere: no shoreline, no banks, no foam lines, no boats, no reflected objects; fills " +
      "the whole frame edge to edge, with no border or empty margin anywhere.",
    subject: "Subject: a water surface texture of {{Subject}}.",
  },
  wall: {
    template: "arda-wall",
    rules:
      "Wall pieces join into long walls on a square grid, so the top of the wall is seen as a " +
      "straight band of even thickness, every arm runs perfectly straight along the image's " +
      "horizontal or vertical centre line and is cut off square exactly at the image edge it " +
      "reaches. The junction is exactly at the centre of the image. Everything that is not wall " +
      "is fully transparent (a PNG with a real alpha channel, background: transparent): no ground, " +
      "no grass, no floor, no shadow.",
    subject: "Subject: one modular wall piece from a grid-map wall kit, {{Subject}}.",
  },
  prop: {
    template: "arda-prop",
    rules: cutoutRules,
    subject: "Subject: {{Subject}}, seen from directly above.",
  },
  vegetation: {
    template: "arda-vegetation",
    rules: `${cutoutRules} No grass, soil or other plants around it.`,
    subject: "Subject: {{Subject}}, seen from directly above.",
  },
};

const templates = Object.fromEntries(
  Object.entries(classes).map(([cls, c]) => [
    c.template,
    {
      kind: "image",
      classes: [cls],
      body: `${style} ${c.subject} {{Shape}} {{Detail}}${c.textured ? " {{Variant}}" : ""} ${c.rules}`,
    },
  ]),
);

// ---------------------------------------------------------------------------------------------
// Ground and water: one description per key, plus variant wording.

const ground = {
  cliff: [
    "a sheer rock cliff face seen from above, layered sedimentary strata in grey and ochre",
    "Bands of strata run left to right across the frame, with cracked ledges, fissures and a little lichen; hard broken rock.",
  ],
  cobbles: [
    "rounded cobblestones set in a town street",
    "Fist-sized grey and brown cobbles in loose rows with dark gritty joints, about 40 cobbles across the frame.",
  ],
  dirt: ["bare brown earth", "Fine dry soil with small clods, a few tiny pebbles and dry twigs."],
  farmland: [
    "a ploughed field with straight parallel furrows running left to right",
    "Rich dark brown turned soil in evenly spaced ridges and troughs, about five furrows across the frame height, a few clods.",
  ],
  flagstone: [
    "a paved courtyard of large irregular flagstones",
    "Pale grey and sandy stone slabs of mixed sizes, a few hairline cracks, thin dark joints with grit.",
  ],
  forest_floor: [
    "forest floor of dark humus, fallen needles, twigs and small leaves",
    "Scattered pine needles, acorn cups, tiny twigs and small moss patches on dark soil.",
  ],
  grass: [
    "short green grass",
    "Lush mid-green turf with small tufts and a few clover leaves, fine blades painted in short strokes.",
  ],
  gravel: ["loose gravel", "Small rounded and angular pebbles in grey, buff and slate tones packed together."],
  heath: [
    "open heathland ground of low heather, dry grass and bare peat",
    "Purple-brown heather tufts mixed with tawny grass and small peaty gaps.",
  ],
  ice: [
    "a frozen surface of thick ice",
    "Pale blue-white ice with fine cracks, trapped air bubbles and a light frosting, slightly glossy.",
  ],
  leaf_litter: [
    "a carpet of fallen autumn leaves",
    "Overlapping brown, russet and yellow leaves with a few twigs and acorns.",
  ],
  marsh: [
    "waterlogged marsh ground",
    "Soggy dark mud with small puddles, tufts of rush and sedge, and patches of moss.",
  ],
  meadow: [
    "a wildflower meadow of long grass",
    "Longer, slightly windswept green grass dotted with tiny white, yellow and purple flowers.",
  ],
  moss: [
    "a thick carpet of moss",
    "Soft cushions of bright and dark green moss with tiny star-like fronds and a few pebbles poking through.",
  ],
  mud: ["wet churned mud", "Glossy dark brown mud with small puddles, ruts and a wet sheen."],
  mudflat: [
    "a tidal mudflat",
    "Smooth grey-brown silt with a wet sheen, shallow rivulets, tiny shells and worm casts.",
  ],
  packed_earth: [
    "hard packed earth",
    "Compacted light brown soil smoothed by traffic, with fine cracks and a few embedded pebbles.",
  ],
  pasture: [
    "grazed pasture grass",
    "Short, even green grass cropped by livestock, a few clover patches and darker tufts.",
  ],
  planks: [
    "a wooden plank floor with the boards running left to right",
    "Oak boards of equal width with fine grain, nail heads in rows and staggered end joints.",
  ],
  reed_bed: [
    "a dense reed bed in shallow water",
    "Crowded upright reed stems seen from above as spiky tufts, with glints of dark water between them.",
  ],
  rock: ["bare rock", "Weathered grey bedrock with cracks, small ledges and spots of lichen."],
  rug: [
    "a large patterned woven carpet covering a floor",
    "A repeating geometric medallion pattern in deep red, indigo and cream with a woven texture; the pattern runs off every edge.",
  ],
  salt_crust: [
    "a dry salt flat",
    "White and pale buff salt crust broken into polygon plates with slightly raised edges.",
  ],
  sand: ["fine dry sand", "Pale golden sand with soft wind ripples and a few tiny shells and darker grains."],
  scree: [
    "loose scree of broken rock",
    "Angular grey rock fragments of mixed sizes packed loosely together.",
  ],
  scrub: [
    "dry scrubland ground",
    "Sparse dry grass, small thorny shrub sprigs and patches of sandy earth.",
  ],
  snow: ["fresh snow", "Soft white snow with subtle blue-grey hollows, faint drifts and a few sparkles."],
  stone_floor: [
    "an indoor floor of square cut stone tiles in a regular grid",
    "Smooth grey stone tiles of equal size with thin mortar lines, about four tiles across the frame height.",
  ],
  trail: [
    "a trodden dirt footpath surface",
    "Trampled light brown earth with small stones and faint wheel marks; no path edges, no grass verges.",
  ],
  stubble: [
    "a harvested grain field of short golden stubble",
    "Cut straw stalks in faint, irregular lines with loose straw and bare soil showing between them.",
  ],
  fallow: [
    "a fallow field resting between crops",
    "Rough brown soil overgrown with weeds, young grass and wild plants; no furrows.",
  ],
  water_deep: [
    "deep open water",
    "Dark teal-blue water with gentle ripples and subtle darker swirls; no bottom visible.",
  ],
  water_shallow: [
    "clear shallow water over a pebbly bed",
    "Light turquoise water with gentle ripples, the sandy, pebbled bottom visible through it.",
  ],
};

// Variant wording. Natural surfaces differ only in arrangement; structured surfaces (one layout
// shared by every variant) differ only in wear and stains.
const naturalVariant = [
  "One patch of this surface with an even, random arrangement.",
  "A different patch of the same surface: same colours, scale and density, a new random arrangement.",
  "Another patch of the same surface: same colours, scale and density, a new random arrangement.",
  "A fourth patch of the same surface: same colours, scale and density, a new random arrangement.",
];
const structuredVariant = [
  "Variant A, the master layout: clean and well kept.",
  "Variant B: exactly the same layout as variant A, only more worn, with grime and a little moss in the joints.",
  "Variant C: exactly the same layout as variant A, stained and damp in places.",
  "Variant D: exactly the same layout as variant A, dusty and a little faded, with a few chips.",
];
const structuredOverrides = {
  planks: [
    "Variant A, the master layout: clean, lightly oiled boards.",
    "Variant B: exactly the same boards as variant A, scuffed and dusty, with a few scratches.",
    "Variant C: exactly the same boards as variant A, with dark spill stains and water marks.",
    "Variant D: exactly the same boards as variant A, sun-faded and paler, with a few dark nail stains.",
  ],
  rug: [
    "Variant A, the master layout: rich colours.",
    "Variant B: exactly the same pattern as variant A, faded and a little threadbare.",
    "Variant C: exactly the same pattern as variant A, with a dark wine stain and some dirt.",
  ],
  farmland: [
    "Variant A, the master layout: freshly turned soil.",
    "Variant B: exactly the same furrows as variant A, a little drier and paler, with a few small weeds.",
    "Variant C: exactly the same furrows as variant A, damp and darker after rain.",
    "Variant D: exactly the same furrows as variant A, with a faint green haze of seedlings along the ridges.",
  ],
  cliff: [
    "Variant A, the master layout: clean rock.",
    "Variant B: exactly the same strata as variant A, with more lichen and a few dark water streaks.",
    "Variant C: exactly the same strata as variant A, weathered and paler.",
  ],
};

// ---------------------------------------------------------------------------------------------
// Walls: the kit gives the material, the role gives the geometry.

const kits = {
  stone: {
    subject: "a 5-foot section of rough-hewn grey stone wall about 2 feet thick, seen from directly above",
    detail: "Irregular grey and buff mortared blocks with pale joints and chipped edges.",
    thickness: "about one fifth of the image height",
    door: "an iron-banded oak door",
    window: "a narrow window with a stone sill and leaded glass",
    gate: "a pair of heavy oak gate leaves",
    post: "a square stone pier",
  },
  timber: {
    subject: "a 5-foot section of timber-framed wall, dark oak beams with pale lime-plastered infill, seen from directly above",
    detail: "A dark oak sill beam along both faces with pale plaster between; pegged joints.",
    thickness: "about one seventh of the image height",
    door: "a plank door of weathered oak",
    window: "a small window with wooden shutters and a plank sill",
    gate: "a pair of broad plank barn doors",
    post: "a square oak post",
  },
  wattle: {
    subject: "a 5-foot section of woven wattle wall, hazel withies on upright stakes partly daubed with clay, seen from directly above",
    detail: "Rows of stake tops with woven withies between, patches of dried clay daub.",
    thickness: "about one eighth of the image height",
    door: "a simple woven hurdle door",
    window: "a small square opening with a twig lattice",
    gate: "a wide woven hurdle gate on rope hinges",
    post: "a thick round stake",
  },
  drystone: {
    subject: "a 5-foot section of low drystone field wall, stacked fieldstones without mortar, seen from directly above",
    detail: "Fitted grey fieldstones with a row of upright coping stones along the top, moss in the gaps.",
    thickness: "about one fifth of the image height",
    door: "a narrow stone stile step",
    window: "a small gap left low in the wall",
    gate: "a wooden five-bar field gate",
    post: "a tall upright gatepost stone",
  },
  hedge: {
    subject: "a 5-foot section of dense clipped hedgerow of hawthorn and hazel, seen from directly above",
    detail: "Leafy green clumps with darker hollows, a few tiny white blossoms, neatly trimmed sides.",
    thickness: "about one quarter of the image height",
    door: "a small wooden wicket gate in a gap",
    window: "a stretch clipped low and thin enough to see through",
    gate: "a wooden five-bar field gate across a gap",
    post: "a thicker rounded clump of hedge",
  },
  palisade: {
    subject: "a 5-foot section of palisade, sharpened upright logs set side by side, seen from directly above as a row of round log tops",
    detail: "Round pointed log tops in a tight row, lashed with rope, bark on, pale cut points.",
    thickness: "about one sixth of the image height",
    door: "a narrow log postern door",
    window: "an arrow slit between two logs",
    gate: "a pair of plank-braced log gate leaves",
    post: "a thicker squared log",
  },
  city_wall: {
    subject: "a 5-foot section of massive fortified city wall of dressed pale limestone, seen from directly above",
    detail: "A paved wall-walk between two crenellated parapets, one along each long side.",
    thickness: "about two fifths of the image height",
    door: "a small iron-studded postern door",
    window: "an arrow loop through one parapet",
    gate: "iron-banded double gates",
    post: "a square buttress wider than the wall",
  },
};

for (const [name, kit] of Object.entries(newKits)) kits[name] = kit;

const across = "it runs from the left edge to the right edge along the horizontal centre line";
const acrossShort = "left edge to right edge on the centre line";
const roles = {
  run: () => `A straight run: ${across}, with no branches and no openings.`,
  door: (k) => `Doorway in a straight run from ${acrossShort}; middle third: a gap between jambs, wall top stopped, shut by ${k.door} along the wall line.`,
  window: (k) => `A straight run with a window: ${across}, with ${k.window} in the middle third.`,
  gate: (k) => `Gateway in a straight run from ${acrossShort}; middle: a gap between posts, wall top stopped, shut by ${k.gate} along the wall line.`,
  post: (k) => `A straight run with a post: ${across}, with ${k.post} at the exact centre.`,
  corner: () =>
    "An L-shaped corner: one arm runs from the centre to the right edge and the other from the centre to the bottom edge; nothing left of or above the centre.",
  tee: () =>
    "A T-junction: arms run from the centre to the left edge, the right edge and the bottom edge; nothing above the centre.",
  cross: () =>
    "A cross junction: arms run from the centre to all four edges (left, right, top and bottom), meeting in a plus shape.",
  end: () =>
    "A wall end: one arm runs in from the right edge and stops at the centre with a finished, capped end; nothing left of, above or below the centre.",
};

// ---------------------------------------------------------------------------------------------
// Props and vegetation: [subject, detail].

const props = {
  altar: [
    "a stone temple altar, a long block of carved pale stone with a linen runner, two candlesticks and a small offering bowl on top",
    "Carved trim along its edges and a few wax drips; its top face is what we see.",
  ],
  anvil: [
    "a blacksmith's iron anvil on a squat wooden stump",
    "Dark forged iron with a bright polished working face and a pointed horn to one side; the round stump shows around it.",
  ],
  armour_stand: [
    "a wooden armour stand holding a steel breastplate, pauldrons and a helmet",
    "The helmet crown at the centre, shoulder plates to either side, the cross-shaped wooden foot showing beneath.",
  ],
  banner: [
    "a free-standing banner pole on a three-legged iron foot, with a deep red banner hanging from its crossbar",
    "The round pole top and finial at the centre, the crossbar and the top of the cloth with a simple gold emblem, the feet spread around.",
  ],
  bar_counter: [
    "a long tavern bar counter of polished dark wood",
    "Tankards, a few mugs, a cloth and a small cask tap on the countertop; a brass foot rail along the front edge.",
  ],
  barrel: [
    "a single upright wooden barrel with iron hoops and a closed lid",
    "A round plank lid with a bung, dark iron hoops at the rim, warm brown oak staves.",
  ],
  bed: [
    "a single wooden bed with a straw mattress, a rumpled wool blanket and a pillow",
    "Pillow and headboard at the top of the frame, a simple oak frame, homespun linen, a folded blanket at the foot.",
  ],
  bench: [
    "a plain wooden plank bench",
    "Two thick oak boards forming the seat, worn smooth in the middle, the legs just showing at the ends.",
  ],
  bookshelf: [
    "a tall wooden bookcase full of books",
    "Its top board, and along the front edge the tops of rows of leather-bound books and a few scrolls; dark walnut.",
  ],
  brazier: [
    "an iron brazier on three legs, its shallow bowl full of glowing coals",
    "Orange coals with bright embers inside a wrought iron rim; the glow stays inside the bowl, with no light spill around it.",
  ],
  bridge_deck: [
    "a 5 by 10 foot strip of timber bridge deck: five narrow planks, each about one foot wide, side by side, seen from above, no rails",
    "Planks run top to bottom; rows of iron nails where they sit on hidden beams; a bolted edge timber across each end; square-cut edges so strips join.",
  ],
  bridge_deck_stone: [
    "one square of stone bridge paving: worn flat paving slabs, seen from directly above, no parapets, no walls, no kerbs",
    "Two or three slabs filling the square edge to edge, mortar joints between them; straight square-cut edges on all four sides so squares join seamlessly.",
  ],
  bucket: [
    "a small wooden bucket with iron bands and a rope handle, half full of water",
    "The water surface inside, the rim of staves and the rope handle arching across.",
  ],
  candle_stand: [
    "a tall iron candle stand with several lit candles",
    "A ring of white candles on a wrought iron crown with small warm flames, the tripod feet visible beneath.",
  ],
  cart: [
    "a two-wheeled wooden farm cart with shafts",
    "Shafts pointing up, a plank bed holding a few sacks, a spoked wheel on each side.",
  ],
  cask_rack: [
    "a wooden rack holding three large casks lying on their sides",
    "The casks lie side by side on a sturdy oak cradle, iron hoops, a tap at each front end.",
  ],
  chair: [
    "a simple wooden chair with a slatted back",
    "A square seat with the back rail along the top edge; worn oak.",
  ],
  chest: [
    "a wooden chest with iron bands and a closed lid",
    "A rectangular lid with iron strapping, corner brackets and a lock plate; dark oak.",
  ],
  crane: [
    "a wooden dockside crane, a timber mast with an angled jib arm, ropes, a pulley and a hook",
    "A square base frame of beams at the centre, the jib reaching toward one corner, a coil of rope; weathered timber and iron.",
  ],
  crate: [
    "a wooden shipping crate with a nailed plank lid",
    "Pale pine planks with diagonal bracing on the lid and nail heads at the corners.",
  ],
  cupboard: [
    "a tall wooden cupboard",
    "Its top board with a moulded rim and the doors along the front edge; dark oak with iron hinges.",
  ],
  dock_planks: [
    "a square section of wooden dock decking",
    "Weathered grey planks running left to right on two beams, iron nails, a few gaps; straight cut edges on all four sides.",
  ],
  fence: [
    "a 5-foot section of split-rail wooden fence running from left to right",
    "A post at each end and two rails between them; the section runs straight across the frame through its centre.",
  ],
  ferry_boat: [
    "a flat-bottomed wooden river ferry, a broad rectangular plank deck with low sides and a rope guide post at each end",
    "Tarred planks, iron cleats and a coil of rope on an empty deck.",
  ],
  ferry_rope: [
    "a ferry rope post, a stout timber post with an iron pulley and a short length of taut rope",
    "The round post top, the pulley wheel and the rope leading off to one side.",
  ],
  forge: [
    "a blacksmith's stone forge hearth with glowing coals and leather bellows beside it",
    "A brick-and-stone hearth with a bed of orange coals, tongs resting on the rim, bellows to one side; no smoke.",
  ],
  grave: [
    "a grave, a low earth mound with a weathered headstone at its head",
    "The headstone at the top end, a few wildflowers on the mound; the stone is blank, with no lettering.",
  ],
  grindstone: [
    "a treadle grindstone, a round sandstone wheel in a wooden frame with a crank",
    "The wheel's rim as a band across the centre, the frame beams around it, a small water trough under it.",
  ],
  hay_bale: [
    "a single rectangular hay bale tied with twine",
    "Golden straw texture, two twine bands, loose wisps at the edges.",
  ],
  haycart: [
    "a wooden farm wagon piled high with loose hay",
    "Shafts at the top, a heaped golden hay load covering most of the bed, wheels at the sides.",
  ],
  hearth: [
    "an indoor stone fireplace hearth with a crackling log fire",
    "A semicircular stone hearth with its back wall along the top edge, logs burning with orange embers, an iron pot hook; no smoke.",
  ],
  ladder: [
    "a wooden ladder lying flat",
    "Two side rails and evenly spaced rungs; worn pale wood.",
  ],
  lantern: [
    "a small iron lantern with a lit candle inside, standing on the ground",
    "A square lantern with glass panes and a ring handle on top; the warm glow stays inside the glass.",
  ],
  loom: [
    "a wooden floor loom with a half-woven piece of cloth",
    "A heavy frame of beams, warp threads running top to bottom, a band of madder-red cloth, a bench along the front.",
  ],
  marker_post: [
    "a wooden waymarker post with a painted band near the top",
    "The square post top with a small cap and a painted red band; no letters or symbols.",
  ],
  market_stall: [
    "a market stall, a striped canvas awning over a wooden counter",
    "Mostly the awning: faded red and cream stripes, sagging a little, with the goods-laden counter edge peeking out at the front.",
  ],
  milestone: [
    "a squat stone milestone with a rounded top",
    "Weathered grey stone with lichen spots; no carved letters or numbers.",
  ],
  millstone: [
    "a large round millstone lying flat",
    "A grey stone disc with a square hole at its centre and dressing grooves running outward in a spiral pattern.",
  ],
  oven: [
    "a domed clay bread oven",
    "A round beehive dome of plastered clay and brick with a small arched mouth at the front glowing with embers.",
  ],
  pew: [
    "a wooden church pew",
    "A long seat with a high back rail along the top edge and carved end panels; dark polished oak.",
  ],
  rowboat: [
    "a small wooden rowboat",
    "Bow at the top, clinker-built planks, two thwarts, a pair of oars resting inside; empty, with no water around it.",
  ],
  rug_small: [
    "a small woven rug",
    "Deep red and indigo with a geometric border and fringed short ends; slightly worn.",
  ],
  sacks: [
    "a small pile of three tied burlap sacks of grain",
    "Rough hessian weave, tied necks, slumped against each other.",
  ],
  shelf: [
    "a wooden wall shelf with jars, pots and folded cloth",
    "A narrow plank shelf with clay jars, a few bottles and bundles lined up along it.",
  ],
  signpost: [
    "a wooden signpost with three pointing arm boards",
    "The square post top at the centre and three plank arms pointing different ways; the boards are blank, with no lettering.",
  ],
  statue: [
    "a stone statue of a robed figure with raised arms on a square plinth",
    "The head and shoulders, raised arms and robe folds above the plinth's square edge; weathered pale marble.",
  ],
  stool: [
    "a round three-legged wooden stool",
    "A round seat cut from a single slab, the leg tips just showing beyond the rim.",
  ],
  table: [
    "a long rectangular wooden table",
    "A heavy planked oak top with a pewter jug, two plates and a candle; edges worn smooth.",
  ],
  tent: [
    "a canvas ridge tent",
    "A pitched canvas roof with its ridge through the centre, guy ropes and pegs at the corners; off-white canvas with mud stains.",
  ],
  throne: [
    "a carved wooden throne with a red cushioned seat",
    "A high back along the top edge with carved finials, armrests and a red velvet cushion; gilded trim.",
  ],
  trough: [
    "a long wooden water trough",
    "A planked box with iron bands, filled with clear water.",
  ],
  weapon_rack: [
    "a wooden weapon rack with spears, swords and a shield",
    "A horizontal frame holding spears and swords in a row, a round shield resting at one end.",
  ],
  well: [
    "a round stone well with a wooden winch frame",
    "A ring of fitted stones around dark water, a crossbeam with a rope winch and a bucket across the centre.",
  ],
  wheelbarrow: [
    "a wooden wheelbarrow",
    "One wheel at the top, two handles pointing down; a plank tray with a little soil in it.",
  ],
  woodpile: [
    "a neat stack of split firewood logs",
    "The round ends and split faces of logs stacked in a rectangular pile, bark on the outer edges.",
  ],
  workbench: [
    "a carpenter's workbench with tools",
    "A thick oak top with a vice at one end, a saw, a mallet, chisels and wood shavings on it.",
  ],
  waterwheel: [
    "a wooden mill water wheel",
    "The wheel reads as a long band of paddles across the centre with its axle running left to right; wet dark timber and iron.",
  ],
  sheep: [
    "a woolly sheep standing",
    "A fluffy cream fleece, the small dark face at the top, ears to the sides.",
  ],
  cow: [
    "a brown and white cow standing",
    "Head at the top: the patched back, the ridge of the spine and short horns.",
  ],
  hen: [
    "a small brown farmyard hen",
    "A rounded russet body, a red comb at the top and tail feathers at the bottom.",
  ],
  stairs: [
    "a short flight of interior wooden stairs",
    "Treads stepping up toward the top of the frame, each a little lighter than the one below, with a handrail along one side.",
  ],
  drain: [
    "a square iron street drain grate set in a stone frame",
    "Dark cast-iron bars over a black gap, a frame of four dressed stones around it.",
  ],
};

const vegetation = {
  boulder: [
    "a single large weathered granite boulder",
    "Rounded, lumpy grey stone with lichen patches and a few cracks; solid and heavy.",
  ],
  bush: [
    "a round leafy green shrub",
    "Dense clusters of small leaves in layered clumps, darker inside, lighter tips catching the light.",
  ],
  bush_flowering: [
    "a round flowering shrub covered in small pink and white blossoms",
    "Dense clusters of small leaves in layered clumps with blossoms scattered over the top.",
  ],
  cattail: [
    "a clump of cattails",
    "Strap-like leaves radiating from the centre with several brown, sausage-shaped seed heads among them.",
  ],
  fallen_log: [
    "a fallen tree trunk lying horizontally",
    "Rough bark with moss and a few broken branch stubs, a pale broken end at each end showing rings.",
  ],
  fern: ["a single fern plant", "Fronds radiating from the centre in a rosette, bright green with darker midribs."],
  flower_patch: [
    "a small tight cluster of wildflowers",
    "Low clumps of daisies, poppies and blue cornflowers with their leaves, loosely round.",
  ],
  heather: [
    "a low mound of flowering heather",
    "Fine needle-like foliage covered in tiny purple-pink flowers, a rounded cushion shape.",
  ],
  juniper: [
    "a juniper shrub",
    "Spiky blue-green foliage in irregular dense tufts with a few dusty blue berries.",
  ],
  lily_pads: [
    "a cluster of floating water lily pads with two white lily flowers",
    "Round notched green pads overlapping loosely; no water painted around them.",
  ],
  mushroom_ring: [
    "a fairy ring of small mushrooms",
    "A loose, broken circle of pale brown and cream toadstool caps, open in the middle.",
  ],
  reeds: [
    "a dense clump of tall reeds",
    "Many slender stems and blade leaves forming a spiky starburst, with feathery tan seed heads.",
  ],
  rock_large: [
    "a large angular rock of grey stone",
    "Several fractured facets and ledges, cracks and patches of moss and lichen.",
  ],
  rock_outcrop: [
    "a big rocky outcrop of layered grey stone",
    "Stepped ledges and broken slabs, deep cracks, tufts of moss in the crevices, an irregular outline.",
  ],
  rock_small: ["a small rock", "A single angular grey stone with a spot of lichen."],
  scree_patch: [
    "a loose patch of scree, many small broken rock fragments",
    "Angular grey stones of mixed sizes, dense in the middle and thinning to scattered stones at the edges.",
  ],
  stones: [
    "a small cluster of four or five stones",
    "Rounded and angular stones in varied grey and buff tones, close together.",
  ],
  stump: [
    "a tree stump",
    "A flat sawn top showing growth rings, rough bark around the rim and a few roots spreading out.",
  ],
  tall_grass: [
    "a tuft of tall wild grass",
    "Long blades radiating from the centre with a few seed heads, green with straw-coloured tips.",
  ],
  tree_alder: [
    "the full canopy of an alder tree",
    "A dense oval crown of dark glossy green leaves in rounded clusters; the trunk is hidden.",
  ],
  tree_birch: [
    "the full canopy of a birch tree",
    "Light, airy foliage of small bright green leaves with glimpses of white branches; the trunk is hidden.",
  ],
  tree_dead: [
    "a dead tree",
    "Bare grey branches radiating from the trunk top in an uneven star, no leaves at all.",
  ],
  tree_elm: [
    "the full canopy of a tall elm tree",
    "A broad crown of dense serrated leaves in big rounded masses; the trunk is hidden.",
  ],
  tree_fruit: [
    "the full canopy of a small apple tree",
    "A rounded crown of green leaves with red apples among them; the trunk is hidden.",
  ],
  tree_oak: [
    "the full canopy of a broad oak tree",
    "Lobed leaf clusters in big rounded masses of rich green, a few gaps showing branches, an irregular round outline.",
  ],
  tree_pine: [
    "the crown of a pine tree",
    "Long needle clusters in layered tiers forming a star-like crown; the trunk is hidden.",
  ],
  tree_spruce: [
    "the crown of a spruce tree",
    "Short dark needles in dense concentric tiers forming a pointed, star-shaped outline.",
  ],
  tree_stunted: [
    "a small stunted, windswept tree",
    "A sparse, twisted crown of small dark leaves leaning to one side.",
  ],
  tree_willow: [
    "the full canopy of a weeping willow",
    "Long trailing fronds cascading outward from the centre, pale silvery green.",
  ],
};

// ---------------------------------------------------------------------------------------------
// Shape keyword: proportions for cut-outs, scale for textures.

function cutoutShape([w, h]) {
  const feet = (n) => n * 5;
  if (w === h)
    return w === 1
      ? "Its outline is compact and about as wide as it is long, fitting one 5-foot square."
      : `Its outline is roughly round or square, about ${feet(w)} feet across.`;
  const long = Math.max(w, h) / Math.min(w, h);
  const ratio = long === 2 ? "twice" : long === 1.5 ? "one and a half times" : `${long} times`;
  return w > h
    ? `Its outline is about ${ratio} as long as it is wide (${feet(w)} by ${feet(h)} feet), lying horizontally across the frame.`
    : `Its outline is about ${ratio} as long as it is wide (${feet(w)} by ${feet(h)} feet), its long axis running top to bottom.`;
}

function textureShape([w]) {
  return `The frame height spans about ${w * 5} feet at true scale for a 5-foot battle-map grid.`;
}

// Square footprints (every texture, wall piece and square cut-out) ask Slopify for 1:1; long
// ones get the matching 16:9 or 9:16 frame, and the importer crops to the footprint.
function formatOf(_cls, [w, h]) {
  if (w === h) return "1:1";
  return h > w ? "9:16" : "16:9";
}

// ---------------------------------------------------------------------------------------------

function heightOf(notes) {
  const match = /height (\d+) ft/.exec(notes);
  return match === null ? null : Number(match[1]);
}

// ---------------------------------------------------------------------------------------------
// Catalogue metadata: what goes into import.toml beside the image.

// What a piece does to movement, sight and cover. Expansion families name one of these.
const blocking = {
  solid: { blocks_movement: true, blocks_sight: false, difficult_terrain: false, cover: "half" },
  tall: { blocks_movement: true, blocks_sight: true, difficult_terrain: false, cover: "three_quarters" },
  total: { blocks_movement: true, blocks_sight: true, difficult_terrain: false, cover: "total" },
  post: { blocks_movement: true, blocks_sight: false, difficult_terrain: false, cover: "none" },
  low: { blocks_movement: false, blocks_sight: false, difficult_terrain: true, cover: "half" },
  rough: { blocks_movement: false, blocks_sight: false, difficult_terrain: true, cover: "none" },
  clear: { blocks_movement: false, blocks_sight: false, difficult_terrain: false, cover: "none" },
  canopy: { blocks_movement: false, blocks_sight: false, difficult_terrain: false, cover: "half" },
  wall: { blocks_movement: true, blocks_sight: true, difficult_terrain: false, cover: "total" },
};

const placeholderById = new Map(placeholder.assets.map((a) => [a.id, a]));
const placeholderByGround = new Map();
for (const a of placeholder.assets) if (a.ground && !placeholderByGround.has(a.ground)) placeholderByGround.set(a.ground, a);

const tagsOf = (t) => ({
  biome: [...t.biome],
  culture: [...t.culture],
  wealth: [...t.wealth],
  function: [...t.function],
  free: [...t.free],
});
const blockingOf = (a) => ({
  blocks_movement: a.blocks_movement,
  blocks_sight: a.blocks_sight,
  difficult_terrain: a.difficult_terrain,
  cover: a.cover === "full" ? "total" : a.cover,
});
const human = (fn, free, wealth = ["poor", "modest"]) => ({ biome: ["temperate"], culture: ["human"], wealth, function: fn, free });

// The few of the 225 the placeholder library does not draw yet.
const extraMeta = {
  "prop.waterwheel": { tags: human(["mill"], ["structure", "water"]), layer: "prop", height_ft: 12, ...blocking.solid },
  "prop.sheep": { tags: human(["farm", "barn"], ["animal", "livestock:sheep"]), layer: "prop", height_ft: 3, ...blocking.rough },
  "prop.cow": { tags: human(["farm", "barn"], ["animal", "livestock:cattle"]), layer: "prop", height_ft: 5, ...blocking.solid },
  "prop.hen": { tags: human(["farm", "barn", "farmhouse"], ["animal", "livestock:poultry"]), layer: "prop", height_ft: 1, ...blocking.clear },
  "prop.stairs": {
    tags: human(["house", "inn", "tavern", "manor", "keep", "library", "warehouse", "temple"], ["structure"]),
    layer: "prop",
    height_ft: 0,
    ...blocking.rough,
  },
  "prop.drain": { tags: human(["street"], ["road"]), layer: "floor", height_ft: 0, ...blocking.clear },
};

function originalMeta(id, groundKey, layer, height) {
  const base = placeholderById.get(id) ?? (groundKey === undefined ? undefined : placeholderByGround.get(groundKey));
  if (base !== undefined)
    return { layer: layer || base.layer, height_ft: height ?? base.height_ft, tags: tagsOf(base.tags), ...blockingOf(base) };
  if (extraMeta[id] !== undefined) {
    const m = extraMeta[id];
    return { layer: m.layer, height_ft: height ?? m.height_ft, tags: tagsOf(m.tags), ...blockingOf(m) };
  }
  if (groundKey !== undefined)
    return { layer: layer || "ground", height_ft: 0, tags: tagsOf(human([], [], [])), ...blocking.clear };
  throw new Error(`no metadata for ${id}`);
}

// Metadata an alt shares with its base, so the two are interchangeable.
const sharedMeta = (base) => ({
  footprint: [...base.footprint],
  pixels: [...base.pixels],
  layer: base.layer,
  height_ft: base.height_ft,
  functions: [...base.functions],
  tags: tagsOf(base.tags),
  ...blockingOf(base),
});

const dirOf = { ground: "ground", water: "ground", wall: "walls", prop: "props", vegetation: "vegetation" };

// Every entry in prompts.json goes through here: one trimmed line per keyword, at most 200
// characters, every template slot filled, and the prompt rendered the way Slopify renders it.
function finish(id, entry) {
  for (const [name, value] of Object.entries(entry.values)) {
    if (value.length > valueMax || /[\n\r]/.test(value) || value.trim() !== value || value.length === 0)
      problems.push(`${id}: ${name} is ${value.length} chars or not one trimmed line`);
  }
  const body = templates[entry.template].body;
  const missing = slotsOf(body).filter((name) => !(name in entry.values));
  if (missing.length) throw new Error(`${id}: no value for ${missing.join(", ")}`);
  entry.prompt = render(body, entry.values);
  return entry;
}

function textureValues(key, variant, structured, footprint) {
  const text = ground[key];
  if (text === undefined) throw new Error(`no ground description for ${key}`);
  const wording = structured ? (structuredOverrides[key] ?? structuredVariant)[variant] : naturalVariant[variant];
  if (wording === undefined) throw new Error(`no variant wording ${variant} for ${key}`);
  return { Subject: text[0], Shape: textureShape(footprint), Detail: text[1], Variant: wording };
}

// ---------------------------------------------------------------------------------------------
// The 225 of the checklist (tier 0: the first run).

function describe(row) {
  const cls = row.class;
  const footprint = pair(row.footprint_squares);
  const pixels = pair(row.pixels);
  const parts = row.id.split(".");
  const groundKey = cls === "ground" || cls === "water" ? parts[1] : undefined;
  const meta = originalMeta(row.id, groundKey, row.layer, heightOf(row.notes));
  const entry = {
    class: cls,
    template: classes[cls].template,
    format: formatOf(cls, footprint),
    footprint,
    pixels,
    layer: meta.layer,
    height_ft: meta.height_ft,
    functions: row.functions
      ? row.functions
          .split(",")
          .map((f) => f.trim())
          .filter(Boolean)
      : [],
    file: row.file || null,
    status: row.status,
    tier: 0,
    tags: meta.tags,
    blocks_movement: meta.blocks_movement,
    blocks_sight: meta.blocks_sight,
    difficult_terrain: meta.difficult_terrain,
    cover: meta.cover,
  };
  if (groundKey !== undefined) {
    const variant = Number(parts[2]);
    const structured = /structured/.test(row.notes);
    entry.ground = groundKey;
    entry.structured = structured;
    entry.variant = variant;
    if (structured && variant > 0) entry.derive_from = `${parts[0]}.${groundKey}.0`;
    entry.values = textureValues(groundKey, variant, structured, footprint);
  } else if (cls === "wall") {
    const [, kitName, role] = parts;
    const kit = kits[kitName];
    if (kit === undefined || roles[role] === undefined) throw new Error(`no wall text for ${row.id}`);
    entry.kit = kitName;
    entry.role = role;
    entry.values = { Subject: kit.subject, Shape: roles[role](kit), Detail: `${kit.detail} The wall band is ${kit.thickness} thick.` };
  } else {
    const name = parts.slice(1).join(".");
    const text = (cls === "prop" ? props : vegetation)[name];
    if (text === undefined) throw new Error(`no ${cls} description for ${row.id}`);
    entry.values = { Subject: text[0], Shape: cutoutShape(footprint), Detail: text[1] };
  }
  return finish(row.id, entry);
}

// ---------------------------------------------------------------------------------------------
// The expansion (expansion.mjs): ground variants, wall variants and kits, cut-out families.

const roleOrder = ["run", "door", "window", "gate", "post", "corner", "tee", "cross", "end"];

function groundExpansion() {
  const out = [];
  for (const [key, [target, tier]] of Object.entries(groundTargets)) {
    const prefix = key.startsWith("water_") ? "water" : "ground";
    const first = assets[`${prefix}.${key}.0`];
    if (first === undefined) throw new Error(`groundTargets names ${key}, which the checklist lacks`);
    const have = Object.values(assets).filter((a) => a.ground === key).length;
    for (let variant = have; variant < target; variant += 1) {
      const id = `${prefix}.${key}.${variant}`;
      const entry = {
        class: first.class,
        template: first.template,
        format: first.format,
        ...sharedMeta(first),
        file: `ground/${id}.png`,
        status: "new",
        tier,
        ground: key,
        structured: first.structured,
        variant,
      };
      if (first.structured) entry.derive_from = `${prefix}.${key}.0`;
      entry.values = textureValues(key, variant, first.structured, first.footprint);
      out.push([id, finish(id, entry)]);
    }
  }
  return out;
}

function wallEntry(id, kitName, role, tier, meta, detail, opening, extra = {}) {
  const kit = kits[kitName];
  const entry = {
    class: "wall",
    template: classes.wall.template,
    format: "1:1",
    ...meta,
    file: `walls/${id}.png`,
    status: "new",
    tier,
    kit: kitName,
    role,
    ...extra,
    values: {
      Subject: kit.subject,
      Shape: roles[role]({ ...kit, ...opening }),
      Detail: `${detail} The wall band is ${kit.thickness} thick.`,
    },
  };
  return [id, finish(id, entry)];
}

function wallExpansion() {
  const out = [];
  // New kits: every piece, with the kit's own tags.
  for (const [kitName, kit] of Object.entries(newKits)) {
    const meta = {
      footprint: [1, 1],
      pixels: [128, 128],
      layer: "wall",
      height_ft: 10,
      functions: [...kit.fn],
      tags: {
        biome: kit.tags.biome ?? ["temperate"],
        culture: kit.tags.culture ?? ["human"],
        wealth: [],
        function: [...kit.fn],
        free: kit.tags.free ?? [],
      },
      ...blocking.wall,
    };
    for (const role of roleOrder) {
      const id = `wall.${kitName}.${role}`;
      const entry = wallEntry(id, kitName, role, kit.tier, meta, kit.detail, {});
      assets[id] = entry[1];
      out.push(entry);
    }
  }
  // Alts of every piece: two for the run, one for each other role.
  const variants = {
    ...wallVariants,
    ...Object.fromEntries(
      Object.entries(newKits).map(([name, k]) => [
        name,
        { same: k.same, wear: k.wear, door: k.altDoor, window: k.altWindow, gate: k.altGate, post: k.altPost, tier: k.tier },
      ]),
    ),
  };
  for (const [kitName, v] of Object.entries(variants)) {
    if (kits[kitName] === undefined) throw new Error(`wall variants for unknown kit ${kitName}`);
    const opening = { door: v.door, window: v.window, gate: v.gate, post: v.post };
    roleOrder.forEach((role, at) => {
      const baseId = `wall.${kitName}.${role}`;
      const base = assets[baseId];
      if (base === undefined) throw new Error(`no base piece ${baseId}`);
      const wears = role === "run" ? [v.wear[0], v.wear[1]] : [v.wear[(at + 1) % v.wear.length]];
      wears.forEach((wear, n) => {
        const id = `${baseId}.alt${n + 1}`;
        out.push(
          wallEntry(id, kitName, role, v.tier ?? 1, sharedMeta(base), `${v.same}, ${wear}.`, opening, {
            variant_of: baseId,
            derive_from: baseId,
          }),
        );
      });
    });
  }
  return out;
}

function cutoutExpansion() {
  const out = [];
  for (const e of expansion) {
    const cls = e.id.startsWith("veg.") ? "vegetation" : "prop";
    let meta;
    if (e.variant_of !== undefined) {
      const base = assets[e.variant_of];
      if (base === undefined) throw new Error(`${e.id}: no base ${e.variant_of}`);
      if (base.class !== cls) throw new Error(`${e.id}: class differs from ${e.variant_of}`);
      meta = sharedMeta(base);
    } else {
      const m = e.meta;
      if (blocking[m.block] === undefined) throw new Error(`${e.id}: unknown block ${m.block}`);
      const footprint = [...m.fp];
      meta = {
        footprint,
        pixels: footprint.map((n) => n * 128),
        layer: m.layer ?? "prop",
        height_ft: m.h,
        functions: [...m.fn],
        tags: {
          biome: m.biome ?? ["temperate"],
          culture: m.culture ?? ["human"],
          wealth: m.wealth ?? (cls === "prop" ? ["poor", "modest"] : []),
          function: [...m.fn],
          free: m.free ?? [],
        },
        ...blocking[m.block],
      };
    }
    const entry = {
      class: cls,
      template: classes[cls].template,
      format: formatOf(cls, meta.footprint),
      ...meta,
      file: `${dirOf[cls]}/${e.id}.png`,
      status: "new",
      tier: e.tier,
    };
    if (e.variant_of !== undefined) entry.variant_of = e.variant_of;
    entry.values = { Subject: e.subject, Shape: cutoutShape(meta.footprint), Detail: e.detail };
    if (assets[e.id] !== undefined) throw new Error(`duplicate id ${e.id}`);
    assets[e.id] = finish(e.id, entry);
    out.push([e.id, assets[e.id]]);
  }
  return out;
}

// Generation order: tier, then visual impact, then rounds (every first alt before any second
// alt), then the order of the source. `--tier 1 --limit 200` therefore takes the 200 that
// change maps the most.
const frequent = new Set(
  [
    "barrel", "crate", "sacks", "table", "bench", "bed", "chair", "chest", "cart", "stool", "shelf",
    "cupboard", "lantern", "bucket", "woodpile", "hay_bale", "fence", "market_stall", "tent", "rug_small",
  ].map((n) => `prop.${n}`),
);
function impact(id, a) {
  if (a.class === "ground" || a.class === "water") return 0;
  if (a.class === "vegetation") return a.layer === "canopy" ? 1 : 4;
  if (a.class === "wall") return a.role === "run" ? 2 : 5;
  return frequent.has(a.variant_of ?? id) ? 3 : 6;
}
// The nth new variant of a ground key counts as round n, like the nth alt of a cut-out.
const originalVariants = {};
function round(id, a) {
  const alt = /\.alt(\d+)$/.exec(id);
  if (alt !== null) return Number(alt[1]);
  return a.ground === undefined ? 0 : a.variant - originalVariants[a.ground] + 1;
}

// ---------------------------------------------------------------------------------------------

const problems = [];
const rows = readChecklist(checklistPath);
const assets = {};
for (const row of rows) {
  if (assets[row.id] !== undefined) throw new Error(`duplicate id ${row.id}`);
  assets[row.id] = describe(row);
}
const originals = Object.keys(assets);
for (const id of originals) {
  const g = assets[id].ground;
  if (g !== undefined) originalVariants[g] = (originalVariants[g] ?? 0) + 1;
}
const added = [...groundExpansion(), ...wallExpansion(), ...cutoutExpansion()];
const seen = new Set(originals);
for (const [id] of added) {
  if (seen.has(id)) throw new Error(`duplicate id ${id}`);
  seen.add(id);
}
const order = added.map(([id, a], at) => ({ id, a, key: [a.tier, impact(id, a), round(id, a), at] }));
order.sort((x, y) => {
  for (let i = 0; i < x.key.length; i += 1) if (x.key[i] !== y.key[i]) return x.key[i] - y.key[i];
  return 0;
});
const ordered = Object.fromEntries([...originals.map((id) => [id, assets[id]]), ...order.map(({ id, a }) => [id, a])]);

if (problems.length) {
  console.error(problems.join("\n"));
  process.exit(1);
}

const tierCounts = {};
for (const a of Object.values(ordered)) tierCounts[a.tier] = (tierCounts[a.tier] ?? 0) + 1;

const pack = {
  format_version: 1,
  about:
    "Arda tactical art prompts for Slopify. Built by build-prompts.mjs from checklist.csv (the first 225) and expansion.mjs (the rest); edit those, not this file.",
  checklist: "checklist.csv",
  expansion: "expansion.mjs",
  keywords: ["Subject", "Shape", "Detail", "Variant (ground and water only)"],
  keyword_max_chars: valueMax,
  style,
  classes: Object.fromEntries(
    Object.entries(classes).map(([cls, c]) => [cls, { template: c.template, rules: c.rules }]),
  ),
  templates,
  tiers: {
    counts: tierCounts,
    about:
      "0: the first 225 (already generated). 1: every variant of those 225, which fixes repetition on every map. 2: building-function and biome sets. 3: culture sets, rare dressing and the new wall kits. Within a tier, assets run in order of visual impact.",
  },
  notes: {
    structured:
      "Structured ground variants must share one layout. A model cannot be relied on to repeat a layout, so generate variant .0 first and either derive the others from it by hand (tint, wear and stains in an image editor) or generate them with --reference pointing at the accepted .0.",
    walls:
      "The importer centre-crops a wall canvas to a square and turns it to the canonical arms (run W-E, corner E+S, tee E+S+W, end E, cross all four). The prompts ask for arms that reach the image edges, so the square middle of a 16:9 frame keeps them. Wall alts keep the base piece's band: generate them with --reference pointing at the accepted base piece (derive_from).",
    variants:
      "<id>.altN is a further take of <id>, which the compositor picks among by seed. Alts share their base's footprint, layer and tags exactly, so any take fits any place the base fits. generate.mjs saves them as <id>__altN.png so the importer numbers them in the same order.",
    sizes:
      "The importer crops each cut-out to its silhouette and scales it to fill its footprint, so relative size comes from the footprint, not from the prompt.",
  },
  assets: ordered,
};

const text = `${JSON.stringify(pack, null, 2)}\n`;
const count = Object.keys(ordered).length;
if (process.argv.includes("--check")) {
  const current = readFileSync(outPath, "utf8");
  if (current !== text) {
    console.error("prompts.json is out of date: run node tools/art-gen/build-prompts.mjs");
    process.exit(1);
  }
  console.log(`prompts.json is up to date (${count} assets).`);
} else {
  writeFileSync(outPath, text);
  console.log(`wrote ${outPath} (${count} assets; by tier ${JSON.stringify(tierCounts)})`);
}
