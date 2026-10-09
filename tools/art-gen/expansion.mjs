// The library expansion: every asset beyond the 225 in checklist.csv. build-prompts.mjs turns
// these records into prompts.json entries; edit descriptions here and rebuild.
//
// Ids follow arda-art-import's naming, so the importer files each image where it belongs:
//
//   ground.<key>.<n>    a further texture variant of a ground key (n continues the series)
//   <id>.alt<n>         a further take of a cut-out or wall piece. The compositor picks among an id
//                       and its alts by seed, so an alt must be interchangeable with its base: the
//                       same footprint, layer and tags, differing only in wear, contents, banding,
//                       shape or colour. Alts therefore carry no metadata of their own.
//   prop.<name>, veg.<name>, wall.<kit>.<role>
//                       new content; a piece that suits only one wealth level, culture or biome
//                       is its own id (prop.table_wealthy, prop.bed_dwarf), never an alt
//
// Family metadata (short forms, expanded by build-prompts.mjs):
//   fp      footprint [w, h] in squares          layer   ground|water|floor|prop|wall|canopy
//   h       height_ft                            block   a key of `blocking` in build-prompts.mjs
//   fn      function tags (vocabulary only)      wealth, culture, biome   controlled tags
//   free    free tags
//
// Controlled tags must be in the catalogue vocabulary (assets/tactical/placeholder/catalog.json):
// biome temperate|boreal|alpine|wetland, culture human, wealth poor|modest|wealthy. Arid and coastal
// pieces and the dwarf, elf and orc sets carry free tags (`arid`, `coastal`, `dwarf`, `elf`, `orc`),
// with an empty biome or culture list, so a bare tag query finds them today and `biome:temperate`
// or `culture:human` leaves them out.
//
// Tiers: 1 is every variant of the original 225 (they fix repetition on every map at once), 2 the
// building-function and biome sets, 3 the culture sets, rare dressing and the new wall kits, 4 the
// underground set (arda-dungeon): cave and bedrock ground, the cave wall kit, stairs down, torch
// sconces and stalagmites.

export const entries = [];

// Further takes of an id: `<base>.alt<n>`, numbered from 1. Items are [subject, detail].
function alts(base, tier, items) {
  items.forEach(([subject, detail], i) => {
    if (subject.length === 0 || detail.length === 0) throw new Error(`${base}: empty alt text`);
    entries.push({ id: `${base}.alt${i + 1}`, variant_of: base, tier, subject, detail });
  });
}

// A new id and its alts: the first item is the id itself.
function fam(id, meta, tier, items) {
  if (items.length < 3) throw new Error(`${id}: give every new asset at least three takes`);
  const [[subject, detail], ...rest] = items;
  entries.push({ id, meta, tier, subject, detail });
  alts(id, tier, rest);
}

// Short metadata builders.
const M = (fp, h, block, fn, more = {}) => ({ fp, h, block, fn, ...more });
const tree = (fp, h, biome, free, more = {}) => ({ fp, h, layer: "canopy", block: "canopy", fn: [], biome, free: ["tree", ...free], ...more });
const plant = (fp, h, block, biome, free, more = {}) => ({ fp, h, block, fn: [], biome, free, ...more });
const dwarf = { culture: [], free: ["dwarf"] };
const elf = { culture: [], free: ["elf"] };
const orc = { culture: [], free: ["orc"] };
const arid = { biome: [], free: ["arid"] };
const with_ = (base, more) => ({ ...base, ...more, free: [...(base.free ?? []), ...(more.free ?? [])] });

// ---------------------------------------------------------------------------------------------
// Ground and water: how many variants each key should have in all, and the tier of the new ones.
// build-prompts.mjs adds `ground.<key>.<n>` from the checklist's count up to this target.

export const groundTargets = {
  grass: [4, 1],
  meadow: [4, 1],
  dirt: [4, 1],
  forest_floor: [4, 1],
  pasture: [4, 1],
  sand: [4, 1],
  mud: [4, 1],
  rock: [4, 1],
  snow: [4, 1],
  scrub: [4, 1],
  heath: [4, 1],
  gravel: [4, 1],
  trail: [4, 1],
  packed_earth: [4, 1],
  leaf_litter: [4, 1],
  moss: [4, 1],
  marsh: [4, 1],
  scree: [4, 1],
  water_shallow: [4, 1],
  water_deep: [4, 1],
  cobbles: [4, 1],
  flagstone: [4, 1],
  planks: [4, 1],
  stone_floor: [4, 1],
  farmland: [4, 1],
  fallow: [3, 1],
  stubble: [3, 1],
  cliff: [3, 1],
  ice: [3, 1],
  mudflat: [3, 1],
  reed_bed: [3, 1],
  rug: [3, 1],
  salt_crust: [3, 1],
};

// Ground keys the checklist lacks, complete: [subject, detail], how many variants and the tier.
export const newGround = {
  cave_floor: {
    tier: 4,
    variants: 3,
    text: [
      "the damp floor of a natural cave",
      "Packed grey-brown grit over uneven rock, small loose pebbles, a few flat stones and darker damp patches; walkable ground.",
    ],
  },
  bedrock: {
    tier: 4,
    variants: 3,
    text: [
      "solid dark rock mass seen from above, the unbroken stone around underground passages",
      "Very dark charcoal grey stone with faint broad plates and fine cracks, low contrast and nearly featureless.",
    ],
    block: "wall",
  },
};

// ---------------------------------------------------------------------------------------------
// Walls. Every role of every kit gets alts that keep the plain run's band (material, colour,
// thickness) and differ in wear, so pieces still join along one wall: two for `run`, one for the
// other roles. Doors, gates, windows and posts also vary their opening.

export const wallVariants = {
  stone: {
    same: "The same rough-hewn grey and buff mortared blocks as the plain run",
    wear: [
      "a few blocks cracked and patched with paler mortar",
      "moss and a little ivy creeping along one face",
      "darker rain-stained blocks and a chipped top course",
      "lichen spots and one larger squared cornerstone",
    ],
    door: "a dark red plank door with iron hinges",
    window: "a wide window with closed blue-painted shutters on a stone sill",
    gate: "grey plank gate leaves with iron studs",
    post: "a round stone pier with a cap stone",
  },
  timber: {
    same: "The same dark oak sill beams and pale lime plaster as the plain run",
    wear: [
      "the plaster patched in one place and a little weathered",
      "a pegged repair joint in one beam, plaster slightly greyer",
      "ochre-washed plaster and darker, older beams",
      "a crack across the plaster and a little moss at the sill",
    ],
    door: "a green-painted ledged plank door",
    window: "a small window with a flower box of red geraniums on its sill",
    gate: "broad ox-blood red plank barn doors",
    post: "a round oak post with an iron band",
  },
  wattle: {
    same: "The same hazel withies on stakes as the plain run",
    wear: [
      "with more clay daub covering the weave in the middle",
      "the daub cracked and a few withies poking loose",
      "freshly woven pale withies with little daub",
      "grey weathered withies with moss on the stakes",
    ],
    door: "a grey plank door on withy hinges",
    window: "a small opening with a woven reed screen",
    gate: "a wide hurdle gate of pale split hazel",
    post: "a forked stake lashed with rope",
  },
  drystone: {
    same: "The same fitted grey fieldstones and upright coping as the plain run",
    wear: [
      "one coping stone tilted, more moss in the gaps",
      "yellow lichen spots and a tuft of grass on top",
      "paler, flatter stones and a few gaps",
      "a fern growing from a gap and dark wet stones",
    ],
    door: "a stone step stile with a worn slab",
    window: "a square sheep creep hole low in the wall",
    gate: "a weathered grey five-bar gate with an iron latch",
    post: "a squared granite gatepost with an iron hinge pin",
  },
  hedge: {
    same: "The same dense clipped hawthorn and hazel as the plain run",
    wear: [
      "sprinkled with small white blossom",
      "a little shaggier, with a few red haws and a bramble tendril",
      "freshly clipped, flat-topped and bright green",
      "with honeysuckle twined through and a few yellow leaves",
    ],
    door: "a small green-painted wicket gate",
    window: "a stretch with a gap trimmed into a low arch",
    gate: "a pale new five-bar field gate across a gap",
    post: "a tall clipped hedge pillar, rounded on top",
  },
  palisade: {
    same: "The same row of round pointed log tops as the plain run",
    wear: [
      "one log newer and paler, rope lashings renewed",
      "bark peeling and a little moss on the cut points",
      "charred tips on a few logs and darker wood",
      "an extra lashing band and one shorter log",
    ],
    door: "a narrow iron-strapped log postern",
    window: "a loophole with a small plank shutter",
    gate: "a pair of log gate leaves with a heavy crossbar",
    post: "a tall watch-post log with a square cap",
  },
  city_wall: {
    same: "The same paved wall-walk between crenellated pale limestone parapets as the plain run",
    wear: [
      "a few paving slabs cracked",
      "grass and moss in the paving joints",
      "rain-darkened stone and a repaired merlon",
      "a weathered parapet with chipped crenels",
    ],
    door: "a small dark blue iron-studded postern",
    window: "a wider gun-loop through one parapet",
    gate: "a lowered portcullis grid of dark iron bars",
    post: "a round bastion stump wider than the wall",
  },
};

// New kits, complete. arda-town picks `log` and `adobe` by biome and culture (logic/09
// §building-materials); the `*_partition` kits draw interior partitions. See plan.md.
export const newKits = {
  adobe: {
    tier: 3,
    subject: "a 5-foot section of thick adobe wall, sun-dried mud brick under smooth ochre plaster, seen from directly above",
    detail: "A rounded plaster top in warm ochre and sand tones; a few bare mud bricks show where the plaster has flaked.",
    thickness: "about one quarter of the image height",
    door: "a weathered blue-painted plank door",
    window: "a small deep window with a turned wooden grille",
    gate: "studded plank gate leaves painted dark green",
    post: "a squat rounded adobe buttress",
    tags: { biome: [], culture: [], free: ["arid"] },
    fn: ["house", "farmhouse", "cottage", "inn", "tavern", "bakery", "workshop", "market", "stall"],
    same: "The same ochre-plastered adobe as the plain run",
    wear: ["cracks in the plaster and a darker rain streak", "freshly whitewashed in pale cream", "a bare patch of mud bricks and sand at the foot", "a row of wooden roof-beam ends poking through the top"],
    altDoor: "a faded turquoise door with iron studs",
    altWindow: "a small window with a blue wooden shutter",
    altGate: "a pair of plank gate leaves painted rust red",
    altPost: "a tall adobe pillar with a rounded cap",
  },
  log: {
    tier: 3,
    subject: "a 5-foot section of log cabin wall, round horizontal logs stacked and chinked with clay, seen from directly above",
    detail: "The top log runs the length of the band, bark-brown, with pale clay and moss chinking along both faces.",
    thickness: "about one sixth of the image height",
    door: "a heavy plank door painted barn red",
    window: "a small shuttered window with a log sill",
    gate: "a pair of broad plank doors painted barn red",
    post: "a thick upright corner log",
    tags: { biome: ["boreal", "alpine"], free: [] },
    fn: ["house", "farmhouse", "cottage", "lumber_camp", "stable", "barn", "waystation", "inn"],
    same: "The same stacked round logs with clay chinking as the plain run",
    wear: ["silver-grey weathered logs", "darker tarred logs and fresh pale chinking", "moss along the chinking and a split log", "a log with a knot and peeling bark"],
    altDoor: "a deep green plank door with a latch",
    altWindow: "a small window with carved white trim",
    altGate: "a pair of plank doors painted deep green",
    altPost: "a squared corner post with dovetailed log ends",
  },
  // Tier 5: thin interior partitions (arda-town tags an edge inside one
  // building `partition`; the compositor draws `<kit>_partition` pieces when
  // the library has them and squeezes the kit's own pieces until then).
  timber_partition: {
    tier: 5,
    subject: "a 5-foot section of thin interior partition wall, a single timber stud frame with plastered panels, seen from directly above",
    detail: "A narrow cream plaster band between two slim dark oak sole plates; plainer and much thinner than an outer wall.",
    thickness: "about one twelfth of the image height",
    door: "a narrow plank door",
    window: "a small interior hatch with a wooden shutter",
    gate: "a wide opening with a plain timber lintel",
    post: "a slim square oak post",
    tags: { biome: [], culture: [], free: ["partition"] },
    fn: ["house", "cottage", "inn", "tavern", "bakery", "workshop", "manor"],
    same: "The same thin plastered stud partition as the plain run",
    wear: ["a scuffed lower edge and a patched panel", "freshly limewashed white", "a darker smoke stain near one end", "one stud showing through cracked plaster"],
    altDoor: "a ledged door painted green",
    altWindow: "a small hatch with a sliding board",
    altGate: "an opening hung with a heavy wool curtain",
    altPost: "a slim turned oak post",
  },
  wattle_partition: {
    tier: 5,
    subject: "a 5-foot section of thin interior wattle screen, woven hazel rods daubed with clay, seen from directly above",
    detail: "A narrow band of woven hazel with pale clay daub pressed between the rods; light and much thinner than an outer wall.",
    thickness: "about one fourteenth of the image height",
    door: "a hanging hide flap over a gap",
    window: "a small gap in the weave with a twig lattice",
    gate: "a wide gap closed by a woven hurdle",
    post: "a slim upright hazel stake",
    tags: { biome: [], culture: [], free: ["partition"] },
    fn: ["house", "cottage", "farmhouse", "stall"],
    same: "The same thin daubed wattle screen as the plain run",
    wear: ["daub flaking off a few rods", "fresh grey clay daub", "a sagging stretch of loose weave", "straw mixed through the daub"],
    altDoor: "a lashed hurdle door",
    altWindow: "a gap with a woven reed shutter",
    altGate: "a wide gap with a rough plank board",
    altPost: "a forked hazel stake",
  },
  adobe_partition: {
    tier: 5,
    subject: "a 5-foot section of thin interior adobe partition, a single course of mud brick under smooth plaster, seen from directly above",
    detail: "A narrow rounded band of pale ochre plaster with a few hairline cracks; plainer and much thinner than an outer wall.",
    thickness: "about one tenth of the image height",
    door: "a low plank door",
    window: "a small arched niche through the wall",
    gate: "a wide arched opening",
    post: "a slim plastered pilaster",
    tags: { biome: [], culture: [], free: ["arid", "partition"] },
    fn: ["house", "cottage", "farmhouse", "inn", "tavern", "bakery", "workshop"],
    same: "The same thin ochre-plastered adobe partition as the plain run",
    wear: ["a fresh coat of cream limewash", "a chipped corner showing mud brick", "a faint hand-painted blue band", "a darker damp patch at one end"],
    altDoor: "a turquoise plank door",
    altWindow: "a small niche with a clay lamp ledge",
    altGate: "an arched opening hung with a striped curtain",
    altPost: "a slim pilaster with a rounded cap",
  },
  cave: {
    tier: 4,
    subject: "a 5-foot section of natural cave wall, a ridge of rough dark rock seen from directly above",
    detail: "Irregular grey-brown boulders fused into one craggy band, damp dark cracks and a few pale mineral streaks.",
    thickness: "about one third of the image height",
    door: "a rough plank door wedged into the rock",
    window: "a narrow natural crack through the rock",
    gate: "crude timber gate leaves",
    post: "a rounded rock pillar",
    tags: { biome: [], culture: [], free: ["cave", "dungeon"] },
    fn: [],
    same: "The same craggy band of dark fused boulders as the plain run",
    wear: ["glistening wet patches and a little green slime", "a few pale crystal flecks in the rock", "a cracked boulder with grit at its foot", "white mineral crust streaking down one face"],
    altDoor: "a door of grey weathered planks",
    altWindow: "a thin fissure with a sliver of darkness through it",
    altGate: "a crude gate of lashed poles",
    altPost: "a squat column of fused rock",
  },
};

// ---------------------------------------------------------------------------------------------
// Props: alts of the 225's props (tier 1). Interchangeable takes: same kind, different look.

alts("prop.barrel", 1, [
  ["a single upright wooden barrel with its lid off, full to the brim with dark rainwater", "Iron hoops, warm brown oak staves, a still water surface with a floating leaf inside the rim."],
  ["a single upright wooden barrel with its lid off, heaped with red and green apples", "Iron hoops, warm brown oak staves, round apples mounded above the rim."],
  ["a single upright ale barrel with a brass tap and a chalk-white lid", "Dark iron hoops, honey-brown staves, a round lid with a bung, the tap poking out at one side."],
  ["a single upright weathered grey barrel with rusty hoops and a cracked lid", "Sun-bleached staves, one hoop slipped a little, a split across the lid."],
  ["a single upright wooden barrel with its lid off, packed with salted fish in coarse salt", "Silver fish tails and white salt crystals inside the rim; iron hoops, brown staves."],
  ["a single upright tar-blackened barrel with a sealed lid", "Glossy black pitch over the staves, iron hoops, a few drips of tar down the side."],
]);
alts("prop.crate", 1, [
  ["an open wooden crate packed with straw and green glass bottles", "Pale pine planks, the lid off, bottle necks poking out of golden straw."],
  ["an open wooden crate full of cabbages and turnips", "Rough pine slats with gaps, round green and purple vegetables heaped inside."],
  ["a wooden crate with its lid tied down under a stained canvas sheet", "Canvas folded over the top with rope crossed and knotted; plank corners show at the edges."],
  ["a slatted wooden crate with gaps between the boards", "Narrow pale slats with dark gaps, nailed battens at the corners; empty and light."],
  ["two small wooden crates stacked, the top one turned at an angle", "Pale pine planks with nail heads, the upper crate offset so both lids show."],
  ["a dark oak crate with rope handles and a branded square on the lid", "Heavy dark planks, iron corner brackets, a burnt square mark without letters."],
]);
alts("prop.sacks", 1, [
  ["a pile of four bulging flour sacks dusted white", "Pale linen sacks with tied necks, a haze of flour on the weave; slumped together."],
  ["two burlap sacks, one open with potatoes spilling out", "Rough hessian weave, round brown potatoes tumbling from the open neck."],
  ["a stack of tied grain sacks on a low wooden pallet", "Hessian sacks laid in two crossing rows on pale pallet boards."],
  ["three open sacks rolled down at the top, full of beans, lentils and red spice", "Rolled hessian rims showing brown beans, orange lentils and a deep red spice powder."],
]);
alts("prop.table", 1, [
  ["a rough trestle table of two split planks on crossed legs", "Bare grey planks with a knot hole and a wooden bowl; crude and well worn."],
  ["a long wooden kitchen table with a loaf, a knife and a chopping board", "Scrubbed pale pine top, flour dust, a bunch of onions at one end."],
  ["a long wooden table covered by a cream linen cloth, set with plates and cups", "The cloth hangs a little over the edges; pewter plates, two cups and a jug."],
  ["a long bare wooden table with a candle stub and a scatter of crumbs", "Dark scarred oak with knife marks and wax drips."],
  ["a long wooden table with a sewing basket, folded cloth and scissors", "Honey-coloured top, a spool of thread and a half-mended shirt."],
  ["a long wooden table with a jug, tankards and a wheel of cheese", "Worn oak with wet rings, three pewter tankards and a cut cheese."],
]);
alts("prop.bench", 1, [
  ["a plain wooden bench made from a split half-log on two stout legs", "The flat face up and worn smooth, bark still on the edges."],
  ["a wooden bench with a low back rail", "Two oak seat boards with a back rail along the top edge; worn pale in the middle."],
  ["a weathered grey plank bench", "Silvery weathered boards with a little lichen and a cracked end."],
  ["a painted green wooden bench with a folded blanket on it", "Flaking green paint over pine, a brown wool blanket folded at one end."],
]);
alts("prop.bed", 1, [
  ["a low straw pallet bed on a plank frame with a thin grey blanket", "Lumpy straw ticking, a coarse blanket, no pillow; plain."],
  ["a single wooden bed with a patchwork quilt and a plump pillow", "Pillow at the top, a quilt of faded blue, green and red squares, a simple oak frame."],
  ["a narrow wooden bed covered in thick fur pelts", "Grey and brown pelts over a plank frame, a rolled fur pillow at the top."],
  ["a rumpled single bed with a brown blanket thrown back", "Pillow at the top, creased linen, a pair of boots at the foot."],
  ["a single wooden bed with a blue-striped wool blanket tucked in neatly", "Pillow at the top, a pine frame, a folded shirt at the foot."],
]);
alts("prop.chair", 1, [
  ["a rough wooden chair of split branches lashed together", "Bark still on the legs, a plank seat; crude and rustic."],
  ["a wooden chair with a woven rush seat", "Golden rush weave on the seat, a ladder back along the top edge; pale ash."],
  ["a carved high-backed wooden chair with a red cushion", "Dark oak with carved finials along the back rail."],
  ["a three-legged chair with a short curved back", "Pale wood, a round seat and a short curved back rail."],
  ["a painted blue wooden chair with a cloth draped over the back", "Flaking blue paint, a spindle back along the top edge, a grey cloth."],
]);
alts("prop.chest", 1, [
  ["a small iron-bound strongbox with a heavy padlock", "Dark iron plates riveted over oak, a big lock at the front edge."],
  ["a travel trunk with a domed lid and leather straps", "Brown leather over wood, two buckled straps, brass corner caps."],
  ["a plain pine box chest with rope handles", "Pale knotty planks, a simple lid, frayed rope handles at the ends."],
  ["a sea chest painted dark blue with rope handles", "Sloped sides, a flat lid with a painted compass star, rope beckets."],
  ["a wooden chest with its lid open, holding folded blankets and clothes", "Dark oak and iron bands, the lid back, wool in muted colours inside."],
]);
alts("prop.cart", 1, [
  ["an empty two-wheeled wooden cart with shafts", "Shafts pointing up, a bare plank bed with straw wisps, a spoked wheel on each side."],
  ["a two-wheeled wooden cart loaded with three barrels", "Shafts pointing up, barrels lashed with rope in the bed, spoked wheels at the sides."],
  ["a two-wheeled cart loaded with crates under a canvas tarp", "Shafts pointing up, a lashed grey canvas over a square load, wheels at the sides."],
  ["a two-wheeled cart piled with cut firewood", "Shafts pointing up, split logs heaped in the bed, spoked wheels at the sides."],
]);
alts("prop.stool", 1, [
  ["a square four-legged wooden stool", "A thick square seat, the leg tops showing at the corners; worn dark oak."],
  ["a tall stool with a round seat and a foot ring", "Round dark seat, an iron foot ring showing below the rim."],
  ["a log-section stool, a short round of tree trunk standing on end", "The sawn top shows growth rings, bark around the rim."],
]);
alts("prop.cupboard", 1, [
  ["a tall painted cupboard in faded blue with flowers on the doors", "Its top board and the doors along the front edge; painted folk flowers."],
  ["a rough tall pine cupboard with one door hanging ajar", "Knotty planks, a leather hinge, a glimpse of crockery inside."],
  ["a tall oak cupboard with a jug and a basket on top", "Its top board with a stoneware jug and a small basket; iron hinges along the front edge."],
]);
alts("prop.shelf", 1, [
  ["a wooden wall shelf stacked with folded cloth and blankets", "A narrow plank shelf with neat piles of folded wool in muted colours."],
  ["a wooden wall shelf of round cheese wheels and wrapped sausages", "Golden cheese wheels in a row, sausages and a ham in muslin."],
  ["a wooden wall shelf of tools: saws, mallets and chisels", "Tools laid in a row, a coil of twine and a pot of nails."],
  ["a wooden wall shelf of plates, bowls and cups", "Stacked earthenware plates and bowls, a row of cups along the front edge."],
]);
alts("prop.bookshelf", 1, [
  ["a tall wooden bookcase, half full, with a candle and loose papers", "Its top board, and along the front edge leather book spines leaning in gaps; dark oak."],
  ["a tall bookcase of rolled scrolls in pigeonholes", "A grid of square cubbies along the front edge, each holding scroll ends; pale cedar."],
  ["a tall bookcase crammed with mismatched books and a small globe on top", "Its top board with a globe, spines in green, brown and red along the front edge."],
]);
alts("prop.hearth", 1, [
  ["an indoor stone fireplace hearth with a cooking pot hanging over the fire", "Its back wall along the top edge, a black iron pot on a hook, logs and embers below; no smoke."],
  ["a cold indoor stone fireplace hearth full of grey ash and charred logs", "Its back wall along the top edge, an iron fire dog, soot stains; no fire."],
  ["a brick fireplace hearth with a low fire and a kettle on a trivet", "Red-brown bricks, its back wall along the top edge, a copper kettle; no smoke."],
]);
alts("prop.bar_counter", 1, [
  ["a long worn tavern bar counter with a row of mugs and a rag", "Scarred brown planks, dents and wet rings, a brass foot rail along the front edge."],
  ["a long tavern bar counter with a barrel tap and a plate of bread", "Dark oak top, a small cask with a tap, crusts and a knife."],
  ["a long tavern bar counter with wine jugs and a pewter tray", "Polished honey-coloured top, three jugs, a stack of cups."],
]);
alts("prop.cask_rack", 1, [
  ["a wooden rack holding two large casks and a small keg", "Big casks side by side on an oak cradle, a keg on top; iron hoops, brass taps."],
  ["a wooden rack of six small kegs in two rows", "Stacked kegs on their sides with chalk-white heads facing front; iron hoops."],
  ["a wooden rack of three dark tarred casks with wooden taps", "Black-brown staves, rope-wrapped taps, a drip tray below."],
]);
alts("prop.lantern", 1, [
  ["a round tin lantern with punched star holes and a lit candle inside", "A pierced tin drum with a ring handle; the warm glow stays inside."],
  ["an oil lantern with a brass base and a glass chimney, lit", "A round brass reservoir, a small flame inside the glass; the glow stays inside."],
  ["a hexagonal iron lantern with amber glass, lit", "Six amber panes, a pointed iron cap with a ring; the glow stays inside."],
]);
alts("prop.bucket", 1, [
  ["a wooden bucket full of milk", "White milk in a ring of staves, iron bands and a rope handle."],
  ["an empty wooden bucket lying on its side", "Dark wet staves, iron bands, the open mouth facing one side."],
  ["a pair of wooden buckets on a shoulder yoke", "Two water-filled buckets joined by a curved wooden yoke across the middle."],
]);
alts("prop.woodpile", 1, [
  ["a round stack of split firewood around a central pole", "Log ends and split faces radiating outward, bark on the outside."],
  ["a loose heap of split logs and kindling", "Jumbled split logs, chips and a hand axe stuck in one log."],
  ["a neat stack of split birch firewood", "White papery bark on the log ends, pale split faces."],
]);
alts("prop.hay_bale", 1, [
  ["two rectangular hay bales stacked crosswise", "Golden straw texture, twine bands, the top bale turned across the lower one."],
  ["a single round hay bale lying flat", "A spiral of tightly rolled golden hay seen from its round end."],
  ["a single greener, fresher rectangular hay bale", "Green-gold straw, three twine bands, a few clover heads."],
]);
alts("prop.trough", 1, [
  ["a long hollowed log trough with water", "A split tree trunk scooped out, bark on its sides, water inside."],
  ["a long stone trough with water and green moss", "Carved grey stone, a lip of moss, clear water inside."],
  ["a long wooden trough with a little oat feed and straw", "Grey planks with iron corners, a thin layer of grain, no water."],
]);
alts("prop.rug_small", 1, [
  ["a small woven rug in green and gold stripes", "Bands of moss green and ochre with fringed short ends; lying flat, no floor around it."],
  ["a small oval braided rag rug", "A coil of braided cloth in faded red, blue and brown; lying flat."],
  ["a small rug with a central star medallion in blue and cream", "A woven border and fringed ends; lying flat, no floor around it."],
  ["a small brown and white cow-hide rug", "An irregular hide outline with patches of brown and white; lying flat."],
]);
alts("prop.fence", 1, [
  ["a 5-foot section of split-rail wooden fence running from left to right, weathered grey", "A post at each end and two rails between them, lichen on the posts; straight across the frame's centre."],
  ["a 5-foot section of split-rail wooden fence running from left to right, one rail broken", "A post at each end, one rail whole and one snapped and sagging; straight across the frame's centre."],
  ["a 5-foot section of three-rail wooden fence running from left to right, pale new wood", "A squared post at each end, three sawn rails; straight across the frame's centre."],
]);
alts("prop.tent", 1, [
  ["a patched canvas ridge tent", "A pitched roof with its ridge through the centre, darker canvas patches, guy ropes at the corners."],
  ["a grey-green canvas ridge tent with its door flap tied open", "A pitched roof with the ridge through the centre, pegs and taut ropes."],
  ["a brown oiled-canvas ridge tent with a bedroll airing on the roof", "A pitched roof with the ridge through the centre, a blanket draped over it."],
]);
alts("prop.market_stall", 1, [
  ["a produce market stall under a faded green and white striped awning", "Mostly the awning, with crates of apples, cabbages and carrots peeking out at the front edge."],
  ["a cloth merchant's stall under a blue awning", "Mostly the awning, with bolts of coloured cloth stacked at the front edge."],
  ["a fishmonger's stall under a grey canvas awning", "Mostly the awning, with silver fish on ice and a set of scales at the front edge."],
  ["a potter's stall under a rust-red awning", "Mostly the awning, with jugs, bowls and pots lined up at the front edge."],
  ["a spice and herb stall under a saffron-yellow awning", "Mostly the awning, with open sacks of coloured spices and bundles of herbs at the front edge."],
]);
alts("prop.well", 1, [
  ["a round stone well with a small shingled roof over its winch", "A ring of fitted stones, a square roof of dark shingles covering the middle."],
  ["a low round stone well with a bucket resting on its rim", "A ring of mossy stones around dark water, a wooden bucket and rope on the rim."],
  ["a square timber-framed well with a plank cover and a crank", "A square box of planks, half the cover open on dark water, an iron crank handle."],
]);
alts("prop.brazier", 1, [
  ["a tall bronze brazier bowl on a fluted stand, full of burning coals", "Green-tinged bronze rim, orange coals; the glow stays inside the bowl."],
  ["a low iron fire basket on short legs, with burning logs", "A cage of iron bars holding split logs and flames; no smoke."],
  ["a stone fire bowl full of embers", "A round carved grey stone bowl, dull red embers and ash."],
]);
alts("prop.candle_stand", 1, [
  ["a single tall brass candlestick with a thick lit candle", "A round brass base and drip pan, a white candle with a small flame."],
  ["a cluster of melted candles of different heights on a stone slab", "Pooled wax, small flames, a few burnt-out stubs; votive offerings."],
  ["a three-armed iron candelabrum with lit candles", "Three curving arms with cream candles and small flames, a round foot."],
]);
alts("prop.statue", 1, [
  ["a stone statue of an armoured warrior leaning on a sword, on a square plinth", "Helm, pauldrons and the sword pommel above the plinth's square edge; weathered grey granite."],
  ["a stone statue of a robed woman holding a sheaf of wheat, on a square plinth", "Head, shoulders and the wheat sheaf above the plinth edge; pale sandstone."],
  ["a bronze statue of a seated scholar with an open book, on a square plinth", "Green-patinated bronze, a bowed head and the book's pages, the plinth edge."],
]);
alts("prop.banner", 1, [
  ["a free-standing banner pole with a deep blue banner on its crossbar", "The round pole top and finial at the centre, the cloth top with a simple silver star emblem."],
  ["a free-standing banner pole with a green and white quartered banner", "The pole top and crossbar at the centre, the cloth top with a simple tree emblem."],
  ["a free-standing banner pole with a black and gold chevron banner", "The pole top and crossbar at the centre, a gold chevron on black cloth."],
]);
alts("prop.grave", 1, [
  ["a fresh grave, a mound of dark turned earth with a plain wooden marker", "A simple wooden post at the head end, a few flowers; no lettering."],
  ["an old sunken grave with a tilted, lichen-covered headstone", "Grass grown over the low mound, a moss-covered stone at the head; no lettering."],
  ["a grave covered by a flat carved stone slab", "A long grey slab with a carved border of vines; no lettering."],
  ["a grave with a stone cross and a low iron railing around it", "A low rail of black iron around the mound, a weathered stone cross at the head; no lettering."],
]);
alts("prop.altar", 1, [
  ["a stone altar with a deep blue runner, a brass bowl and white candles", "Pale carved stone, a blue cloth along its length, two lit candles."],
  ["a plain wooden altar table with a white cloth and a bowl of fruit", "Oak with a linen cloth, a wooden bowl of apples, a single candle."],
]);
alts("prop.rowboat", 1, [
  ["a small wooden rowboat painted blue and white", "Bow at the top, painted planks, two thwarts, a pair of oars inside; empty, no water around it."],
  ["an old weathered rowboat with a coil of rope and a bailing bucket", "Bow at the top, grey planks, a tarred patch; empty, no water around it."],
  ["a small wooden fishing boat with a folded net in the stern", "Bow at the top, tarred planks, a net and a fish basket inside; no water around it."],
]);
alts("prop.signpost", 1, [
  ["a wooden signpost with two pointing arm boards", "The square post top at the centre and two plank arms at right angles; blank boards, no lettering."],
  ["a leaning old signpost with one broken arm", "The post top at the centre, two arms, one snapped short; weathered grey, no lettering."],
]);
alts("prop.weapon_rack", 1, [
  ["a wooden weapon rack of polearms: halberds and long spears", "A horizontal frame holding the polearms side by side, iron heads at one end."],
  ["a wooden rack of bows and quivers of arrows", "Unstrung longbows lying in a row, two quivers of fletched arrows at one end."],
]);
alts("prop.armour_stand", 1, [
  ["a wooden armour stand holding a mail shirt and a round helmet", "The helmet at the centre, grey mail draped over the shoulders, the cross-shaped foot."],
  ["a wooden armour stand with a padded gambeson and a kettle helm", "A brimmed helmet at the centre, a quilted tan jacket over the shoulders."],
]);
alts("prop.anvil", 1, [
  ["a blacksmith's anvil on a squat iron-banded log, with a hammer on its face", "Dark forged iron with a bright working face, a hammer lying across it."],
  ["a small square-horned anvil on a stone block", "A blocky grey anvil with a short horn, set on a rough granite block."],
]);
alts("prop.forge", 1, [
  ["a blacksmith's brick forge with a hood edge and a glowing coal bed", "Red-brown bricks, orange coals, a poker and tongs, bellows to one side; no smoke."],
  ["a blacksmith's stone forge with dull red embers and a water bucket", "Grey stone hearth, banked embers, tongs and a bucket beside it; no smoke."],
]);
alts("prop.workbench", 1, [
  ["a cluttered workbench covered in wood shavings, planes and clamps", "A thick oak top with a vice, curls of shavings and a half-made chair leg."],
  ["a long workbench with a pole lathe at one end", "A wooden pole lathe, turned bowls and a pile of chips on an oak top."],
]);
alts("prop.loom", 1, [
  ["a wooden floor loom with a half-woven blue cloth", "A heavy frame of beams, warp threads top to bottom, a band of indigo cloth, a bench along the front."],
  ["a wooden floor loom with a striped green and cream cloth", "Beams and warp threads, a band of striped cloth, a shuttle on the bench."],
]);
alts("prop.grindstone", 1, [
  ["a hand-cranked grindstone on a stone base with a water drip", "The wheel's rim as a band across the centre, a wooden frame, a crank handle."],
  ["a small grindstone on a wooden bench with a blade on it", "A grey wheel in a short frame, a knife resting on the bench."],
]);
alts("prop.oven", 1, [
  ["a domed clay bread oven with its iron door closed", "A round beehive dome of plastered clay, a small arched iron door at the front edge."],
  ["a square brick baker's oven with a glowing mouth and a wooden peel", "A flat brick top, an arched mouth at the front edge glowing orange, a long peel beside it."],
]);
alts("prop.pew", 1, [
  ["a plain wooden pew with no carving", "A long seat with a straight back rail along the top edge; pale pine."],
  ["a dark oak pew with a hymn book and a folded cloth on the seat", "Carved end panels, a back rail along the top edge, a small book."],
  ["a worn pew with a red cushion along its seat", "Polished oak, a back rail along the top edge, a long red cushion."],
]);
alts("prop.throne", 1, [
  ["a gilded throne with a high back and a purple velvet seat", "Gold leaf over carved wood, a high back along the top edge, lion-paw feet."],
  ["a dark oak throne with a blue cushion and carved wolves on the arms", "A high back along the top edge with a carved crest, iron studs."],
]);
alts("prop.haycart", 1, [
  ["a wooden farm wagon half-loaded with hay and a pitchfork on top", "Shafts at the top, a low golden load, a pitchfork lying across it."],
  ["a wooden farm wagon loaded with sheaves of wheat", "Shafts at the top, bundled golden sheaves stacked in rows, wheels at the sides."],
]);
alts("prop.wheelbarrow", 1, [
  ["a wooden wheelbarrow full of cobblestones", "One wheel at the top, two handles pointing down, grey stones heaped in the tray."],
  ["a wooden wheelbarrow full of manure and straw", "One wheel at the top, handles pointing down, a dark heap with a pitchfork."],
]);
alts("prop.ladder", 1, [
  ["a wooden ladder lying flat with one rung missing", "Two weathered side rails and rungs, a gap where one rung is gone."],
  ["a rough ladder of two poles with lashed rungs", "Bark-on poles, rungs tied with rope at each end."],
]);
alts("prop.sheep", 1, [
  ["a woolly sheep grazing with its head down", "A fluffy cream fleece, the small dark face at the top, ears to the sides."],
  ["a black-faced sheep standing", "A greyish fleece, a black face and legs at the top, curled small horns."],
  ["a freshly shorn sheep standing", "A thin cream body with short stubbly wool, a small pale face at the top."],
]);
alts("prop.cow", 1, [
  ["a black and white cow standing", "Head at the top: the patched back, the ridge of the spine and short horns."],
  ["a russet-red cow lying down", "Head at the top turned slightly, legs tucked under a solid red-brown body."],
  ["a shaggy long-horned ginger cow standing", "Head at the top with long curved horns and a fringe, a thick ginger coat."],
]);
alts("prop.hen", 1, [
  ["a small white farmyard hen", "A rounded white body, a red comb at the top and tail feathers at the bottom."],
  ["a strutting rooster with a tall red comb and dark green tail", "Head at the top, copper hackles, sweeping dark tail feathers at the bottom."],
  ["a speckled grey hen with three yellow chicks", "A rounded speckled body, fluffy chicks huddled beside her."],
]);
alts("prop.stairs", 1, [
  ["a short flight of interior stone stairs", "Grey stone treads stepping up toward the top of the frame, worn in the middle, a plain stone side."],
  ["a short flight of rough wooden stairs without a handrail", "Plank treads stepping up toward the top of the frame, each a little lighter than the one below."],
]);
alts("prop.dock_planks", 1, [
  ["a square section of wooden dock decking with a patch of newer planks", "Planks left to right on two beams, the same width as the plain section, three paler new boards; straight cut edges, no rails."],
  ["a square section of wooden dock decking stained with tar and fish scales", "Planks left to right on two beams, dark tar drips and silver scales; straight cut edges on all four sides, no rails."],
  ["a square section of salt-bleached dock decking with a rusty iron mooring ring", "Pale grey planks left to right on two beams, an iron ring bolted near one edge; straight cut edges, no rails."],
]);
alts("prop.bridge_deck", 1, [
  ["a 5 by 10 foot strip of weathered timber bridge deck: five narrow planks, each about one foot wide, side by side, seen from above, no rails", "Planks run top to bottom; nail rows where they sit on hidden beams; grey wood with moss in the joints; square-cut edges so strips join."],
  ["a 5 by 10 foot strip of new timber bridge deck: five narrow pale planks, each about one foot wide, side by side, seen from above, no rails", "Planks run top to bottom; bright nail rows over hidden beams; one plank darker; square-cut edges so strips join."],
  ["a 5 by 10 foot strip of tarred timber bridge deck: five narrow dark planks, each about one foot wide, side by side, seen from above, no rails", "Planks run top to bottom; nail rows over hidden beams; mud and straw on the boards; square-cut edges so strips join."],
]);
alts("prop.bridge_deck_stone", 1, [
  ["one square of mossy stone bridge paving: worn flat slabs, seen from directly above, no parapets, no walls, no kerbs", "Two or three grey slabs edge to edge, moss in the joints; straight square-cut edges on all four sides."],
  ["one square of cart-worn stone bridge paving: flat slabs with two shallow wheel ruts, no parapets, no kerbs", "Slabs filling the square edge to edge, ruts running top to bottom; straight square-cut edges on all four sides."],
]);
alts("prop.milestone", 1, [
  ["a tall slim stone milestone with a pointed top", "Weathered buff stone with moss at its foot; no carved letters or numbers."],
  ["an ancient leaning milestone half covered in moss", "A rounded grey stone tilted to one side; no carved letters or numbers."],
]);
alts("prop.marker_post", 1, [
  ["a wooden waymarker post with a cairn of stones at its foot", "The square post top with a painted white band, small grey stones heaped around it."],
  ["a round wooden waymarker post with a painted yellow cap", "A round post top, a yellow-painted cap and a notch; no letters or symbols."],
]);
alts("prop.drain", 1, [
  ["a round iron street drain cover set in a ring of stones", "A round grate of radial cast-iron bars over a black gap, a ring of dressed stones."],
  ["a rusty square drain grate with leaves caught in it", "Orange-brown iron bars, a few wet leaves, four mossy frame stones."],
]);
alts("prop.ferry_boat", 1, [
  ["a flat-bottomed wooden river ferry carrying a small cart", "A broad plank deck with low sides, a two-wheeled cart lashed in the middle, rope posts at the ends."],
  ["a weathered grey river ferry with a hand-winch at one end", "A broad plank deck with low sides, a wooden winch drum and coiled rope."],
]);
alts("prop.ferry_rope", 1, [
  ["a ferry rope post, a squared timber post with a big iron ring and taut rope", "The square post top, the ring and the rope leading off to one side."],
  ["a ferry rope post braced with a diagonal strut and a pulley", "The round post top, a strut, the pulley wheel and rope leading off to one side."],
]);
alts("prop.crane", 1, [
  ["a wooden treadwheel crane, a big walking wheel beside a mast and jib", "A large wooden wheel at the centre, the jib reaching toward one corner, rope and hook."],
  ["a small wooden dockside derrick with a hanging cargo net", "A square base of beams, a short jib toward one corner, a net of sacks on the hook."],
]);
alts("prop.millstone", 1, [
  ["a pair of millstones, one leaning upright against the other", "Grey stone discs with dressing grooves, a square eye at each centre."],
  ["a cracked old millstone with moss", "A grey disc split across, worn grooves, moss in the eye."],
]);
alts("prop.waterwheel", 1, [
  ["a wooden undershot water wheel with broad paddles", "The wheel reads as a long band of flat paddles across the centre, its axle running left to right; mossy timber."],
  ["a wooden water wheel with iron-rimmed buckets", "A long band of wooden buckets across the centre, iron rims, the axle left to right."],
]);

// ---------------------------------------------------------------------------------------------
// Props: new families, by building function (tier 2 unless noted).

// Inn and tavern.
fam("prop.table_round", M([1, 1], 3, "solid", ["inn", "tavern", "house", "manor", "library"], { free: ["furniture"] }), 2, [
  ["a small round wooden table with two tankards and a candle", "A round oak top worn smooth, wet rings from mugs, a short candle in a holder."],
  ["a small round wooden table with a game board, dice and a pipe", "A round dark top, a chequered board without letters, scattered dice."],
  ["a small round wooden table with a bowl of stew, bread and a spoon", "A round pine top, a steaming wooden bowl, a heel of bread."],
  ["a small round wooden table with playing cards and copper coins", "A round scarred top, a fanned deck without symbols, coins and a mug."],
  ["a small round wooden table with an empty jug and a tipped cup", "A round top with a spill, a cloth, a few crumbs."],
]);
fam("prop.bottle_shelf", M([2, 1], 6, "tall", ["inn", "tavern", "brewery", "manor"], { free: ["storage", "furniture"] }), 2, [
  ["a tall back-bar shelf of wine bottles and stoneware jugs", "Its top board, and rows of bottle tops and jugs along the front edge; dark wood."],
  ["a tall back-bar rack of wine bottles lying in diamond cubbies", "Its top board, round bottle ends in a lattice along the front edge."],
  ["a tall back-bar shelf of tankards, mugs and a keg", "Its top board, hanging pewter tankards along the front edge."],
]);
fam("prop.stage", M([2, 2], 1, "rough", ["inn", "tavern", "market", "street"], { free: ["structure", "floor"] }), 3, [
  ["a low square wooden stage platform with a stool and a lute on it", "Plank boards left to right, a trim edge, a stool and a lute; square-cut corners."],
  ["a low square wooden stage platform with a drum and a music stand", "Plank boards left to right, a small drum, a stand; square-cut corners."],
  ["a bare low square stage platform with a worn centre", "Plank boards left to right, scuffed in the middle; square-cut corners."],
]);
fam("prop.keg_tap", M([1, 1], 4, "solid", ["inn", "tavern", "brewery"], { free: ["container"] }), 3, [
  ["a keg on a wooden stand with a tap and a drip bucket", "A barrel on its side on a cradle, a brass tap over a small bucket."],
  ["a small keg on a stool with a wooden spigot", "A barrel on its side, a wooden tap, a wet patch below."],
  ["two kegs on a wooden cradle with brass taps", "Two barrels side by side on their sides, taps at the front."],
]);

// Bakery.
fam("prop.bread_rack", M([2, 1], 5, "solid", ["bakery", "market", "stall"], { free: ["storage", "food"] }), 2, [
  ["a wooden bread rack of loaves, rolls and braided buns", "Its top board, rows of golden loaves along the front edge."],
  ["a wooden cooling rack of round dark rye loaves", "Slatted shelves, round loaves dusted with flour along the front edge."],
  ["a wooden rack of long baguette loaves standing in baskets", "Its top board, tall baskets of long golden loaves along the front edge."],
]);
fam("prop.kneading_trough", M([2, 1], 3, "solid", ["bakery", "farmhouse"], { free: ["furniture", "food"] }), 2, [
  ["a long wooden kneading trough on legs with a mound of dough", "A tapered wooden tub, pale dough, a scraper and a dusting of flour."],
  ["a long kneading trough with its lid on and a flour sieve on top", "A closed plank lid, a round sieve, flour dust."],
  ["a long kneading trough with three shaped loaves resting in cloth", "Linen folds holding pale risen loaves."],
]);
fam("prop.flour_bin", M([1, 1], 3, "solid", ["bakery", "mill", "farmhouse"], { free: ["container", "food"] }), 3, [
  ["a wooden flour bin with its lid propped open", "A square box full of white flour, a wooden scoop, flour dust on the rim."],
  ["a wooden flour bin with a closed lid and a sack on top", "A square plank box, a slumped flour sack."],
  ["a barrel-shaped flour bin with a cloth cover", "A round tub, a tied white cloth over the top."],
]);
fam("prop.basket", M([1, 1], 1, "rough", ["market", "stall", "bakery", "farm", "house", "cottage", "market_hall"], { free: ["container"] }), 2, [
  ["a round wicker basket of bread loaves", "A woven willow rim, golden loaves and rolls inside."],
  ["a round wicker basket of red apples", "A woven willow rim, apples heaped inside."],
  ["a round wicker basket of wet laundry", "A woven rim, twisted white and blue linens inside."],
  ["a flat wicker basket of silver fish", "A shallow woven tray, fish laid in rows."],
  ["an empty round wicker basket on its side", "A woven willow basket, the open mouth facing one side."],
  ["a round wicker basket of eggs in straw", "A woven rim, pale brown eggs nested in straw."],
]);

// Brewery.
fam("prop.brew_vat", M([2, 2], 6, "tall", ["brewery", "tavern"], { free: ["container"] }), 2, [
  ["a large round open wooden brewing vat full of frothing wort", "Iron hoops around oak staves, a foamy brown surface, a long paddle across it."],
  ["a large round wooden brewing vat with a plank lid and a ladder against it", "A round plank lid, iron hoops, a short ladder at one side."],
  ["a large round wooden vat of dark still beer with a skimmer", "Iron hoops, a glossy dark surface, a long-handled skimmer."],
]);
fam("prop.brew_kettle", M([2, 2], 5, "solid", ["brewery"], { free: ["hearth"] }), 2, [
  ["a big copper brewing kettle over a brick firebox", "A round copper rim and steaming dark liquid, a brick surround with a glowing fire door; no smoke."],
  ["a big copper brewing kettle with its domed lid on", "A shining copper dome, a pipe at one side, a brick base."],
  ["a big dented copper kettle with green patina, cold", "A round dull copper rim, patina streaks, a cold brick base."],
]);
fam("prop.cask_stack", M([2, 1], 5, "solid", ["brewery", "warehouse", "inn", "tavern", "dock"], { free: ["container", "storage"] }), 2, [
  ["a pyramid stack of barrels lying on their sides", "Three barrels below and two above, round ends facing front, iron hoops."],
  ["a row of four barrels lying on their sides in chocks", "Round ends facing front, wooden chocks under them, chalk-white heads."],
  ["a stack of dark tarred barrels lashed with rope", "Black-brown barrels in two rows, a rope net over them."],
]);
fam("prop.mash_tun", M([1, 1], 4, "solid", ["brewery"], { free: ["container"] }), 3, [
  ["a squat wooden mash tun full of steeping grain", "A round tub of oak staves, a thick porridge of pale grain and a rake."],
  ["a squat mash tun with a false bottom of slats showing", "An empty round tub, a slatted floor, damp grain husks."],
  ["a squat mash tun with a cloth over it and a paddle", "A round tub, a stained linen cover, a wooden paddle."],
]);

// Tannery.
fam("prop.tanning_rack", M([2, 1], 6, "solid", ["tannery"], { free: ["tool", "craft:leatherwork"] }), 2, [
  ["a wooden frame with a cow hide stretched by cords", "A rectangle of poles, a pale brown hide laced taut inside it."],
  ["a wooden frame with a dark deer hide stretched by cords", "Lashed poles, a reddish-brown hide laced taut, a scraper hanging from it."],
  ["a drying line of three hides hung over a pole", "A long pole on two posts, hides draped over it in tan and brown."],
]);
fam("prop.tanning_vat", M([1, 1], 3, "rough", ["tannery"], { free: ["container", "water"] }), 2, [
  ["a square stone-lined tanning pit full of brown bark liquor with a hide in it", "Dark brown liquid, a soaking hide, a wooden paddle across the rim."],
  ["a square tanning pit of white lime water", "Milky white liquid in a stone rim, a hide corner showing."],
  ["a wooden tanning tub of dark red-brown liquor", "Iron-banded staves, a thick dark surface, a long pole."],
]);
fam("prop.hide_pile", M([1, 1], 2, "rough", ["tannery", "market", "lumber_camp"], { free: ["container"] }), 2, [
  ["a stack of folded tanned hides", "Layers of brown and tan leather folded in a pile, edges uneven."],
  ["a pile of raw furs and pelts", "Grey, brown and russet furs heaped loosely."],
  ["a bundle of rolled leather tied with cord", "Three rolls of brown leather, cord knots."],
]);
fam("prop.scraping_beam", M([2, 1], 3, "solid", ["tannery"], { free: ["tool", "craft:leatherwork"] }), 3, [
  ["a sloping wooden fleshing beam with a hide draped over it and a two-handled knife", "A smooth log on two legs, a hide hanging over, a curved scraper."],
  ["a bare fleshing beam with a bucket of scraps", "A pale smooth log on legs, a wooden bucket."],
  ["a fleshing beam with a half-scraped dark hide", "A log on legs, a hide part furred and part bare."],
]);
fam("prop.leather_bench", M([2, 1], 3, "solid", ["tannery", "workshop", "stable"], { free: ["tool", "craft:leatherwork"] }), 3, [
  ["a leatherworker's bench with cut straps, awls and a half-made saddle", "A worn oak top, coils of leather, punches and thread."],
  ["a leatherworker's bench with boots, a last and a mallet", "A worn top, a pair of half-stitched boots."],
  ["a leatherworker's bench with belts, buckles and a stamping tool", "A worn top, brass buckles and tooled strips."],
]);

// Apothecary.
fam("prop.jar_shelf", M([2, 1], 6, "tall", ["apothecary", "temple", "library"], { free: ["storage", "furniture"] }), 2, [
  ["a tall shelf of glass jars, bottles and clay pots of herbs", "Its top board, rows of jar lids and stoppered bottles along the front edge; dark wood."],
  ["a tall apothecary chest of many small square drawers", "Its top board, a grid of tiny drawers with brass pulls along the front edge."],
  ["a tall shelf of coloured potion bottles and dried specimens", "Its top board, red, green and blue bottles and a jar with a lizard along the front edge."],
]);
fam("prop.alchemy_table", M([2, 1], 3, "solid", ["apothecary", "library", "manor"], { free: ["furniture", "tool"] }), 2, [
  ["an alchemist's table with an alembic, flasks, a mortar and a small burner", "A dark oak top, glass tubing, a candle under a flask, scattered herbs."],
  ["a herbalist's table of bundled herbs, a chopping board and a mortar and pestle", "Green sprigs, dried flowers, a stone mortar and twine."],
  ["an alchemist's table with a bubbling retort, a skull and open notes", "Coloured liquids, a candle, papers without letters."],
]);
fam("prop.herb_rack", M([1, 1], 5, "solid", ["apothecary", "cottage", "farmhouse"], { free: ["storage"] }), 2, [
  ["a wooden drying rack hung with bundles of herbs and flowers", "A square frame of slats, upside-down bunches of green and purple herbs."],
  ["a tripod drying tray of mushrooms and roots", "A shallow woven tray of brown caps and gnarled roots on three legs."],
  ["a wooden rack of drying lavender and chamomile", "Purple and white bundles hanging in rows."],
]);
fam("prop.cauldron", M([1, 1], 3, "solid", ["apothecary", "cottage", "house", "inn", "lumber_camp"], { free: ["container", "hearth"] }), 2, [
  ["a black iron cauldron of bubbling green brew on a small fire", "A round black rim, a glowing green surface, small flames below; no smoke."],
  ["a black iron cauldron of stew hanging from a tripod", "A round rim, brown stew, a ladle, three iron legs meeting above."],
  ["an empty cold iron cauldron on three short legs", "A sooty black rim and a dull dark inside."],
]);

// Library and school.
fam("prop.desk", M([2, 1], 3, "solid", ["library", "school", "manor", "guardhouse", "toll_house", "temple", "warehouse"], { free: ["furniture"] }), 2, [
  ["a writing desk with an open ledger, a quill and an inkpot", "A dark oak top with a slanted writing board, a candle and papers."],
  ["a plain clerk's desk with stacked papers, a seal and a coin box", "A pine top, neat piles of parchment, a wax seal and an abacus."],
  ["a scribe's desk with a slanted board, an open book and pots of colour", "Bright pigment pots, brushes and a half-painted page without letters."],
  ["a cluttered desk of maps, letters and a burning candle", "A worn top, rolled papers and a brass candlestick."],
]);
fam("prop.lectern", M([1, 1], 4, "solid", ["library", "temple", "school", "shrine"], { free: ["furniture", "religious"] }), 2, [
  ["a wooden lectern holding a large open book", "A slanted top with an open book, a carved post and a cross-shaped foot."],
  ["a brass eagle lectern with wings holding a large open book", "Polished brass wings spread under the book."],
  ["a plain stone lectern with a closed book and a candle", "A pale stone slope, a leather-bound book."],
]);
fam("prop.scroll_rack", M([2, 1], 6, "tall", ["library", "temple", "school"], { free: ["storage"] }), 2, [
  ["a tall rack of diamond cubbies filled with rolled scrolls", "Its top board, scroll ends in a lattice along the front edge; pale cedar."],
  ["a tall rack of scroll cases with coloured end caps", "Its top board, leather tubes with red and green caps."],
  ["a half-empty scroll rack with a few loose scrolls", "Its top board, sparse cubbies, dust."],
]);
fam("prop.book_pile", M([1, 1], 1, "rough", ["library", "school", "manor", "apothecary"], { free: ["decoration"] }), 2, [
  ["a stack of old leather-bound books with a candle stub", "Five books piled unevenly, worn covers in brown and red, a wax drip."],
  ["a scatter of open books, loose papers and a quill on the floor", "Two books lying open, pages without letters, scattered parchment."],
  ["a neat stack of new books tied with string", "Bright green and blue covers, a string cross."],
]);
fam("prop.globe", M([1, 1], 4, "solid", ["library", "manor", "school"], { free: ["decoration"], wealth: ["modest", "wealthy"] }), 3, [
  ["a large world globe in a carved wooden stand", "A painted sphere of blue seas and green lands without lettering, a brass meridian ring."],
  ["a brass armillary sphere on a stand", "Interlocking brass rings around a small ball."],
  ["a celestial globe of dark blue with gold star dots", "A deep blue sphere with gold specks, no lettering, a wooden stand."],
]);
fam("prop.map_table", M([2, 2], 3, "solid", ["library", "keep", "school", "manor", "barracks"], { free: ["furniture", "craft:cartography"] }), 2, [
  ["a large square map table with an unrolled chart, compasses and weights", "A painted chart of coasts and hills without lettering, brass dividers, candle stubs."],
  ["a large square war table with a terrain map and small carved markers", "Painted hills and rivers without lettering, wooden figures, a dagger."],
  ["a large square table with stacked charts and a lamp", "Rolled maps, a brass lamp, an inkpot."],
]);
fam("prop.school_desk", M([2, 1], 3, "solid", ["school", "temple"], { free: ["furniture", "seating"] }), 2, [
  ["a long pupil's desk-bench with slates and chalk", "A slanted plank desk with three small slates, a bench seat along the front edge."],
  ["a long pupil's desk-bench with open copybooks and inkwells", "A slanted plank desk, sunken inkwells, a bench along the front edge."],
  ["a worn pupil's desk-bench with carved scratches and an apple", "Scarred pale wood, a bench along the front edge."],
]);
fam("prop.easel", M([1, 1], 6, "post", ["school", "workshop", "manor"], { free: ["tool", "craft:painting"] }), 3, [
  ["a wooden easel holding a blank canvas, with a palette and brushes", "Three legs spread, the canvas top edge, a palette of paint dabs on the ledge."],
  ["a wooden easel holding a chalk slate board", "Three legs spread, a dark slate, chalk on the ledge."],
  ["a wooden easel with a half-painted landscape", "Three legs, a canvas of green hills without letters."],
]);

// Barracks and guardhouse.
fam("prop.bunk_bed", M([1, 2], 6, "tall", ["barracks", "guardhouse", "keep", "inn", "dock"], { free: ["furniture"] }), 2, [
  ["a wooden bunk bed with grey blankets", "The top bunk's blanket and pillow at the top, a ladder rail at one side; plain pine."],
  ["a wooden bunk bed with rumpled blankets and a kit bag on top", "The top bunk with a canvas bag and a boot hanging off; pine frame."],
  ["a wooden bunk bed with a straw mattress and a hanging cloak", "Straw ticking on top, a red cloak draped over the end post."],
]);
fam("prop.footlocker", M([1, 1], 2, "low", ["barracks", "guardhouse", "keep", "dock"], { free: ["container"] }), 2, [
  ["a soldier's footlocker of olive-painted wood with iron corners", "A flat lid with a rope handle at each end, a small lock."],
  ["an open footlocker with folded uniform, boots and a whetstone", "The lid back, neat folded cloth and a pair of boots inside."],
  ["a scuffed wooden footlocker with a dented lid", "Worn planks, a bent hasp, a painted band without digits."],
]);
fam("prop.training_dummy", M([1, 1], 6, "post", ["barracks", "keep", "guardhouse"], { free: ["weapons"] }), 2, [
  ["a straw training dummy on a wooden post with a crossbar arm", "A burlap head at the centre, straw poking from cuts, the crossbar arms."],
  ["a wooden pell post, a stout scarred stake for sword practice", "A round post top with deep notch marks, splinters around it."],
  ["a quintain, a pivoting arm on a post with a shield at one end and a sandbag at the other", "The post top at the centre, the arm across the frame."],
]);
fam("prop.archery_target", M([1, 1], 5, "post", ["barracks", "keep", "street", "farm"], { free: ["weapons"] }), 2, [
  ["a round straw archery target on a wooden easel with arrows in it", "Painted red, white and blue rings on straw, three arrows in the target."],
  ["a stack of hay bales with a painted target and arrows", "Golden bales, a round painted target on the front, arrow shafts sticking out."],
  ["a round straw target with no arrows and a torn ring", "Faded painted rings, straw spilling from a tear."],
]);
fam("prop.shield_rack", M([2, 1], 5, "solid", ["barracks", "keep", "guardhouse"], { free: ["weapons", "storage"] }), 2, [
  ["a wooden rack of round shields leaning in a row", "Painted shields in red, blue and plain wood with iron bosses."],
  ["a wooden rack of kite shields and helmets", "Tall pointed shields leaning, helmets on pegs."],
  ["a rack of battered shields and broken spear shafts", "Scarred shields, splintered wood."],
]);
fam("prop.stocks", M([2, 1], 4, "solid", ["guardhouse", "street", "market"], { free: ["structure"] }), 3, [
  ["a set of wooden stocks with holes for hands and feet", "Two heavy boards hinged on posts, an iron padlock, a stool behind."],
  ["a wooden pillory on a post with a small platform", "A board with three holes on a post, a plank step."],
  ["old weathered stocks with a rusted lock", "Grey cracked boards, an orange iron hasp."],
]);
fam("prop.cage", M([1, 1], 6, "tall", ["guardhouse", "keep", "market"], { free: ["structure"] }), 3, [
  ["a square iron cage with a barred door, empty", "A square frame of black iron bars seen from above, a straw-covered floor."],
  ["a round iron cage with a domed top and a hook", "Curved black bars meeting at a ring at the centre."],
  ["a wooden slatted animal cage with a latched door", "Square frame of pale slats, straw inside."],
]);
fam("prop.gaol_cot", M([1, 2], 2, "rough", ["guardhouse", "keep"], { free: ["furniture"], wealth: ["poor"] }), 3, [
  ["a bare plank cot with a thin straw pad and a chain", "Grey boards, straw, an iron chain and shackle at the foot."],
  ["a stone sleeping ledge with a grey blanket", "A rough stone slab, a thin folded blanket."],
  ["a sagging rope cot with a ragged blanket", "Woven rope on a frame, a torn brown blanket."],
]);

// Stable and barn.
fam("prop.saddle_rack", M([1, 1], 4, "solid", ["stable", "barn", "waystation", "barracks"], { free: ["storage"] }), 2, [
  ["a wooden saddle rack holding a brown leather saddle and a blanket", "The saddle's seat at the centre, stirrups hanging to the sides, a striped blanket."],
  ["a wooden peg rack of bridles, halters and coiled lead ropes", "Leather straps and buckles hanging from a beam, rope coils."],
  ["a wooden saddle rack with a worn saddle and saddlebags", "A cracked old saddle, two leather bags hanging from it."],
]);
fam("prop.hay_rack", M([2, 1], 4, "solid", ["stable", "barn", "waystation"], { free: ["hay"] }), 2, [
  ["a wooden hay manger full of hay", "A slatted V-shaped rack holding golden hay, a trough beneath."],
  ["a stone feed trough with oats and a little hay", "A long grey stone box with grain and loose hay."],
  ["an almost empty hay manger with a few wisps", "A slatted rack, a few stalks, chewed edges."],
]);
fam("prop.horse", M([1, 2], 6, "solid", ["stable", "farm", "waystation", "street", "barracks", "keep"], { free: ["animal", "livestock:horse"] }), 2, [
  ["a bay horse standing", "Head at the top, a glossy brown body, black mane and tail along the spine."],
  ["a grey dappled horse standing with a saddle", "Head at the top, a dappled grey back, a brown saddle in the middle."],
  ["a black horse standing", "Head at the top, a sleek black body, a long mane falling to one side."],
  ["a heavy chestnut draft horse with feathered hooves and a collar", "Head at the top, a broad red-brown back, a padded collar."],
  ["a piebald pony standing", "Head at the top, white and brown patches, a shaggy mane."],
]);
fam("prop.manure_pile", M([1, 1], 2, "rough", ["stable", "barn", "farm"], { free: ["container"] }), 3, [
  ["a heap of manure and dirty straw with a pitchfork", "A dark brown mound with straw wisps; a pitchfork stuck in it."],
  ["a steaming compost heap of straw and kitchen scraps", "Brown straw, vegetable peel, a little steam kept small."],
  ["a low manure heap edged with planks", "A dark mound in a plank surround."],
]);
fam("prop.hay_pile", M([2, 2], 4, "rough", ["barn", "stable", "farm"], { free: ["hay"] }), 2, [
  ["a big loose heap of golden hay", "Soft mounded hay with stray wisps at the edges."],
  ["a loose heap of hay with a pitchfork and a sleeping cat", "Golden hay, a pitchfork, a curled ginger cat."],
  ["a heap of pale straw bedding", "Flattened yellow straw, trampled at one side."],
]);
fam("prop.grain_bin", M([1, 1], 4, "solid", ["barn", "mill", "farm", "warehouse"], { free: ["container", "food"] }), 2, [
  ["a square wooden grain bin with its lid open, full of golden wheat", "Plank sides, a hinged lid, a scoop in the grain."],
  ["a round wicker grain store with a thatched cover", "A tall woven basket, its top a cone of thatch."],
  ["a square grain bin with its lid shut and a padlock", "Plank sides, iron strap hinges."],
]);
fam("prop.plough", M([1, 2], 3, "low", ["farm", "barn"], { free: ["tool"] }), 2, [
  ["a wooden plough with an iron share", "The long beam toward the top, two handles at the bottom, the iron blade."],
  ["a heavy wheeled plough", "Two small wheels at the top, the beam and mouldboard, handles at the bottom."],
  ["a light scratch plough of bent wood", "A curved beam toward the top, one handle, a small iron tip."],
]);
fam("prop.harrow", M([2, 2], 1, "rough", ["farm", "barn"], { free: ["tool"] }), 3, [
  ["a square wooden harrow frame with iron teeth lying flat", "A lattice of beams with rows of iron spikes."],
  ["a triangular harrow of beams and wooden pegs", "A triangle of timbers with rows of pegs."],
  ["a brushwood harrow of bundled thorn branches on a frame", "Tangled twigs lashed to a beam."],
]);
fam("prop.tool_rack", M([2, 1], 5, "solid", ["barn", "farm", "smithy", "workshop", "mine", "lumber_camp"], { free: ["tool", "storage"] }), 2, [
  ["a wooden rack of farm tools: rakes, hoes, scythes and pitchforks", "Long handles lying in a row, iron heads at one end."],
  ["a rack of tongs, hammers and punches", "Iron tools hanging in a row on a wooden frame."],
  ["a rack of picks, shovels and sledgehammers", "Long handles in a row, iron heads at one end, a lantern hook."],
]);
fam("prop.chicken_coop", M([1, 1], 4, "solid", ["farm", "farmhouse", "cottage"], { free: ["structure", "livestock:poultry"] }), 2, [
  ["a small wooden chicken coop with a sloped plank roof", "The roof planks seen from above, a little ramp at the front edge."],
  ["a woven wicker chicken coop with a straw-thatched top", "A round thatch top, a small door and a hen on the ramp."],
  ["a small chicken coop with a mossy shingle roof", "Grey shingles seen from above, a ramp and a feed dish."],
]);

// Animals.
fam("prop.pig", M([1, 1], 3, "low", ["farm", "barn", "farmhouse"], { free: ["animal", "livestock:pig"] }), 2, [
  ["a pink pig standing", "Head at the top with big ears and a snout, a round pink body, a curly tail."],
  ["a spotted black and pink pig lying in mud", "A round body with black patches, ears flopped, a little mud."],
  ["a sow with three piglets", "A large pink sow lying, small piglets nestled beside her."],
]);
fam("prop.goat", M([1, 1], 3, "low", ["farm", "barn", "farmhouse"], { free: ["animal", "livestock:goat"], biome: ["temperate", "alpine"] }), 2, [
  ["a white goat standing", "Head at the top with short horns and a beard, a lean white body."],
  ["a brown and black goat standing", "Head at the top with curved horns, a brown body with a black stripe."],
  ["a shaggy grey goat", "Head at the top with backswept horns, a long grey coat."],
]);
fam("prop.dog", M([1, 1], 2, "clear", ["farm", "farmhouse", "house", "street", "inn", "keep", "lumber_camp"], { free: ["animal"] }), 2, [
  ["a sleeping brown dog curled up", "A brown coat, nose tucked under the tail."],
  ["a black and white sheepdog lying with its head up", "Head at the top, a white collar and black back."],
  ["a big grey wolfhound standing", "Head at the top, a long rough grey body."],
]);
fam("prop.cat", M([1, 1], 1, "clear", ["house", "farmhouse", "cottage", "inn", "tavern", "barn", "mill", "library"], { free: ["animal"] }), 3, [
  ["a ginger cat curled up asleep", "An orange striped coat curled into a ring."],
  ["a grey tabby cat sitting", "Head at the top, striped grey back, the tail wrapped around."],
  ["a black cat stretched out", "A sleek black body, white paws."],
]);
fam("prop.ox", M([1, 2], 5, "solid", ["farm", "barn", "mill"], { free: ["animal", "livestock:cattle"] }), 2, [
  ["a brown ox with a wooden yoke on its neck", "Head at the top with wide horns, a heavy brown back."],
  ["a pale cream ox standing", "Head at the top with long horns, a heavy pale body."],
  ["a dark grey ox lying down", "Head at the top, legs folded, a dusty back."],
]);
fam("prop.donkey", M([1, 2], 4, "solid", ["farm", "mill", "market", "waystation", "mine"], { free: ["animal"] }), 2, [
  ["a grey donkey with panniers", "Head at the top with long ears, two baskets slung over the back."],
  ["a brown donkey standing", "Head at the top with long ears, a dark stripe along the spine."],
  ["a grey mule with a pack saddle of sacks", "Head at the top, a wooden pack frame with tied sacks."],
]);
fam("prop.duck", M([1, 1], 1, "clear", ["farm", "farmhouse"], { free: ["animal", "livestock:poultry"], biome: ["temperate", "wetland"] }), 3, [
  ["a white farm duck with an orange bill", "Head at the top, a plump white body."],
  ["a pair of mallard ducks", "Two ducks side by side, the drake's green head at the top."],
  ["a white goose with its neck stretched", "Head at the top, a long white neck, an orange bill."],
]);
fam("prop.deer", M([1, 2], 5, "clear", [], { free: ["animal", "wild", "forest"], culture: [], wealth: [], biome: ["temperate", "boreal"] }), 3, [
  ["a red deer stag standing", "Head at the top with broad antlers, a russet body."],
  ["a doe grazing", "Head at the top lowered, a slender tawny body with a pale rump."],
  ["a wild boar standing", "Head at the top with tusks, a bristly dark brown body."],
]);
fam("prop.camel", M([1, 2], 7, "solid", ["market", "waystation", "stable"], with_(arid, { free: ["animal"] })), 3, [
  ["a camel standing with a saddle blanket", "Head at the top on a long neck, a single hump, a striped blanket."],
  ["a camel lying down with packs", "Head at the top, legs folded, bundled packs on the hump."],
  ["a pale two-humped camel standing", "Head at the top, two shaggy humps, a rope halter."],
]);

// Mill.
fam("prop.grain_hopper", M([1, 1], 5, "solid", ["mill"], { free: ["tool", "food"] }), 3, [
  ["a wooden grain hopper, a square funnel full of wheat over a millstone", "A square timber funnel seen from above, golden grain inside, a stone rim below."],
  ["an empty grain hopper with flour dust", "A square timber funnel, a white dusting, a dark spout."],
  ["a grain hopper with a sack emptying into it", "A square funnel, a tilted open sack pouring grain."],
]);
fam("prop.mill_gear", M([2, 2], 2, "low", ["mill"], { free: ["tool"] }), 3, [
  ["a large wooden mill gear wheel lying flat", "A round wheel with pegged wooden cogs around its rim, an axle hole at the centre."],
  ["a broken wooden gear wheel with missing cogs", "A round wheel, gaps in its cog ring, split spokes."],
  ["a wooden lantern gear of staves between two discs", "Two round discs joined by a ring of round staves."],
]);

// Mine.
fam("prop.mine_cart", M([1, 1], 3, "solid", ["mine"], { free: ["vehicle", "mine"] }), 2, [
  ["an empty iron-bound wooden mine cart on four small wheels", "A square open tub of planks and iron straps, rust and mud on it."],
  ["a mine cart full of grey ore with glints of copper", "A square wooden tub heaped with rubble and green-blue glints."],
  ["a mine cart full of black coal", "A square wooden tub heaped with glossy black lumps."],
  ["a mine cart full of rubble with a pickaxe on top", "A square wooden tub of broken grey stone and a pickaxe."],
]);
fam("prop.mine_rails", M([1, 1], 0, "clear", ["mine"], { layer: "floor", free: ["floor", "mine", "rail"] }), 2, [
  ["a straight 5-foot section of mine cart track running top to bottom, two iron rails on wooden sleepers", "Rails reach the top and bottom edges; four sleepers; no ground, no ballast, nothing else."],
  ["a straight 5-foot section of worn mine cart track running top to bottom, rusty rails on cracked sleepers", "Rails reach the top and bottom edges; one sleeper split; no ground."],
  ["a straight 5-foot section of mine cart track running top to bottom on dark tarred sleepers", "Rails reach the top and bottom edges; iron spikes; no ground."],
]);
fam("prop.ore_pile", M([1, 1], 2, "rough", ["mine", "smithy", "warehouse"], { free: ["rock", "mine"] }), 2, [
  ["a heap of broken ore rock with copper-green streaks", "Angular grey lumps, green and rust streaks."],
  ["a heap of black coal lumps", "Glossy black coal with a little dust."],
  ["a heap of iron ore, reddish-brown rocks", "Dark rust-red angular rocks."],
]);
fam("prop.mine_support", M([1, 1], 8, "post", ["mine"], { free: ["structure", "mine"] }), 2, [
  ["a timber mine support, a square post and a crossbeam from above", "A heavy squared beam across the frame on a post, iron brackets."],
  ["a pair of round pit props with a lantern hanging on one", "Two log posts, a short beam and a lit lantern; the glow stays inside."],
  ["a cracked old mine support propped with an extra wedge", "A dark beam across the frame, a split, a fresh pale wedge."],
]);
fam("prop.mine_shaft", M([2, 2], 10, "total", ["mine"], { free: ["structure", "mine"] }), 3, [
  ["a square mine shaft opening framed with timbers and a windlass over it", "A dark square hole edged by beams, a wooden winch with rope and bucket across it."],
  ["a square mine shaft with a ladder going down and a plank cover half off", "Beams around a black hole, a ladder top."],
  ["a square mine shaft with a head frame and pulley", "Beams around a dark hole, an A-frame with a big pulley wheel."],
]);

// Lumber camp.
fam("prop.log_pile", M([2, 1], 4, "solid", ["lumber_camp", "mill", "warehouse", "farm"], { free: ["container", "log"] }), 2, [
  ["a stack of felled tree trunks lying side by side", "Bark-covered logs running left to right, pale sawn ends with rings."],
  ["a pyramid stack of logs held by stakes", "Logs piled in rows, upright stakes at the ends, sawn ends with rings."],
  ["a stack of debarked pale logs", "Smooth creamy logs running left to right, a few bark strips."],
  ["a single huge felled trunk with a crosscut saw on it", "A broad bark-covered trunk running left to right, a long two-man saw."],
]);
fam("prop.timber_pile", M([2, 1], 3, "solid", ["lumber_camp", "workshop", "warehouse", "mill"], { free: ["container"] }), 2, [
  ["a neat stack of sawn planks with spacer sticks", "Pale boards running left to right in layers."],
  ["a pile of roughly squared beams", "Square timbers with adze marks, stacked crosswise."],
  ["a stack of grey weathered planks under a canvas", "Grey boards, a tarp tied over half of them."],
]);
fam("prop.sawhorse", M([1, 1], 3, "low", ["lumber_camp", "workshop", "barn"], { free: ["tool", "craft:carpentry"] }), 2, [
  ["a wooden sawhorse with a half-sawn plank across it", "The top beam across the frame, splayed legs, sawdust on the plank."],
  ["a wooden sawhorse with a log on it and a bow saw", "The top beam across the frame, a short log and a saw."],
  ["a bare wooden sawhorse", "The top beam across the frame, four splayed legs, sawdust below."],
]);
fam("prop.chopping_block", M([1, 1], 2, "low", ["lumber_camp", "farm", "house", "cottage", "farmhouse"], { free: ["tool"] }), 2, [
  ["a chopping block stump with an axe buried in it", "A sawn round top with rings, an axe handle across it, wood chips."],
  ["a chopping block with split kindling around it", "A round stump top, small split logs scattered."],
  ["a scarred chopping block with a hatchet and a half-split log", "A round top deeply cut, a short hatchet."],
]);
fam("prop.sawpit", M([2, 1], 4, "solid", ["lumber_camp"], { free: ["tool", "craft:carpentry"] }), 3, [
  ["a saw pit trestle with a log on it and a long pit saw", "A heavy frame over a dark trench, a log running left to right, the saw upright."],
  ["a saw pit with a half-sawn log and planks peeling off", "A frame over a trench, a log split into boards."],
  ["an empty saw pit frame over a dark trench", "Two beams left to right, sawdust heaps."],
]);

// Waystation, toll house and roads.
fam("prop.hitching_post", M([2, 1], 4, "solid", ["waystation", "inn", "tavern", "stable", "street", "toll_house"], { free: ["structure"] }), 2, [
  ["a wooden hitching rail on two posts", "A smooth worn rail running left to right, iron rings on the rail."],
  ["a wooden hitching rail with a tied lead rope and a water bucket", "The rail left to right, a rope knotted round it, a bucket by one post."],
  ["a stone-post hitching rail with an iron bar", "Two squat stone posts, a black iron bar left to right."],
]);
fam("prop.toll_barrier", M([2, 1], 4, "solid", ["toll_house", "street", "keep"], { free: ["barrier", "road"] }), 2, [
  ["a wooden toll bar, a long pole resting on a forked post, painted in red and white bands", "The pole runs left to right, a counterweight at one end; no lettering."],
  ["a wooden swing gate barrier on a post with a chain and padlock", "A plank gate running left to right, iron hinges on one post."],
  ["a plain log toll bar on two forked posts", "A peeled log left to right, rope ties."],
]);
fam("prop.notice_board", M([1, 1], 7, "post", ["street", "market", "toll_house", "waystation", "guardhouse", "inn"], { free: ["marker"] }), 2, [
  ["a wooden notice board with a little shingle roof", "The roof planks seen from above, the edges of pinned papers showing; no lettering."],
  ["a post with blank paper notices nailed to it", "The round post top, curling paper corners, nail heads; no lettering."],
  ["a stone pillar with blank posters pasted on it", "A round stone top, peeling paper edges; no lettering."],
]);
fam("prop.campfire", M([1, 1], 1, "rough", ["lumber_camp", "waystation", "farm", "mine"], { free: ["light", "camp"] }), 2, [
  ["a campfire in a ring of stones, burning logs with orange flames", "Grey stones in a circle, crossed logs, glowing embers; no smoke."],
  ["a campfire with a spit and a roasting rabbit", "A ring of stones, flames, two forked sticks holding a spit; no smoke."],
  ["a smouldering campfire of glowing embers in a stone ring", "Red embers and charred log ends, grey ash."],
  ["a cold campfire, a ring of stones with ash and charcoal", "Black charred wood, grey ash; no fire."],
]);
fam("prop.bedroll", M([1, 2], 1, "clear", ["lumber_camp", "waystation", "barracks", "mine"], { layer: "floor", free: ["camp"] }), 2, [
  ["a rolled-out bedroll of grey wool on a brown canvas groundsheet", "A rolled pillow at the top, a rumpled blanket."],
  ["a bedroll of furs with a pack at the head", "Brown and grey pelts, a leather backpack at the top."],
  ["a rolled-out bedroll of green wool, neatly smoothed", "A folded cloak as a pillow at the top."],
]);
fam("prop.camp_pack", M([1, 1], 2, "rough", ["lumber_camp", "waystation", "barracks"], { free: ["container", "camp"] }), 2, [
  ["a pile of travel packs, a bedroll and a waterskin", "Leather backpacks, a rolled blanket, a coil of rope."],
  ["a set of saddlebags and a saddle on the ground", "A worn leather saddle, two bulging bags, a canteen."],
  ["a rolled-up bedroll and a pack tied with straps", "A tight cylinder of blanket, a canvas pack."],
]);
fam("prop.cooking_tripod", M([1, 1], 4, "rough", ["lumber_camp", "waystation", "farm"], { free: ["camp", "hearth"] }), 2, [
  ["an iron cooking tripod over a small fire with a pot hanging from a chain", "Three legs meeting at the centre, a black pot, small flames; no smoke."],
  ["a wooden pole tripod with a kettle over embers", "Three lashed poles, a dented kettle, red embers."],
  ["an iron tripod with a pan and no fire", "Three legs, a hanging frying pan, cold ashes."],
]);
fam("prop.lean_to", M([2, 1], 5, "tall", ["lumber_camp", "waystation"], { free: ["structure", "camp"] }), 2, [
  ["a lean-to shelter of branches and leaves on a pole frame", "A sloping roof of laid branches, the ridge pole along the top edge."],
  ["a lean-to of a tarred canvas sheet stretched from a pole", "A sloping canvas sheet, pegs and ropes along the low edge."],
  ["a lean-to of bark slabs on a pole frame", "Overlapping brown bark sheets, the ridge pole along the top edge."],
]);
fam("prop.tent_small", M([1, 2], 4, "total", ["lumber_camp", "waystation", "barracks"], { free: ["structure", "camp"] }), 2, [
  ["a small two-person canvas ridge tent", "The pitched canvas roof with its ridge top to bottom, guy ropes at the corners."],
  ["a small patched brown ridge tent", "A darker canvas ridge, patched panels, pegs at the corners."],
  ["a small green ridge tent with its flap open", "Green canvas, the ridge top to bottom, a folded flap at the bottom."],
]);
fam("prop.tent_military", M([2, 2], 8, "total", ["barracks", "keep", "guardhouse"], { free: ["structure", "camp"] }), 2, [
  ["a square military tent of olive canvas with a pennant at the peak", "A neat square pyramid roof, taut guy ropes and pegs at the corners."],
  ["a square officer's tent of white canvas with blue trim", "A square pyramid roof with a peak at the centre, blue-edged panels."],
  ["a square supply tent of brown canvas with crates at the door", "A pitched roof, ropes at the corners, crates peeking out at one side."],
]);
fam("prop.pavilion", M([2, 2], 9, "total", ["market", "stall", "street", "keep", "manor"], { free: ["structure"], wealth: ["modest", "wealthy"] }), 3, [
  ["a round pavilion with a striped red and gold canvas roof", "A round cone roof with a peak and pennant at the centre, scalloped edges."],
  ["a round pavilion with a blue and white striped roof", "A round cone roof, a gilt finial, scalloped edges."],
  ["a square pavilion of green silk with tassels", "A square pyramid roof, tasselled corners."],
]);
fam("prop.well_desert", M([1, 1], 3, "solid", ["street", "market", "waystation", "farm"], with_(arid, { free: ["structure", "water"] })), 3, [
  ["a stone well with a pulley frame of pale timber and a clay jar on the rim", "A ring of sandstone blocks, a timber cross-frame with a pulley, a water jar."],
  ["a round mud-brick well with a palm-trunk winch", "Ochre plastered rim, dark water, a rough beam with rope."],
  ["a square stone well with a shade cloth over it", "Sandstone blocks, a faded cloth on four poles."],
]);

// Market and market hall.
fam("prop.display_table", M([2, 1], 3, "solid", ["market", "stall", "market_hall", "street"], { free: ["furniture"] }), 2, [
  ["a trestle table of fruit and vegetables in baskets", "Apples, pears, carrots and cabbages in small baskets on planks."],
  ["a trestle table of clay pots, jugs and bowls", "Rows of glazed and plain earthenware."],
  ["a trestle table of folded cloth and ribbons", "Stacks of folded fabric in reds, blues and greens, rolls of ribbon."],
  ["a trestle table of jewellery and trinkets on velvet", "Rings, brooches and necklaces on a deep blue cloth."],
  ["a trestle table of knives, horseshoes and ironmongery", "Blades, nails, hinges and a few tools laid out in rows."],
]);
fam("prop.scales", M([1, 1], 3, "solid", ["market", "stall", "market_hall", "warehouse", "toll_house", "mill"], { free: ["tool"] }), 3, [
  ["a merchant's balance scale on a small table with brass weights", "Two brass pans on a beam, a row of weights."],
  ["a hanging steelyard scale on a tripod with a sack", "A long iron arm, a sliding weight, a hooked sack."],
  ["a big platform scale for sacks", "A flat wooden platform, an iron beam and weights."],
]);

// Shrine and temple.
fam("prop.shrine_stone", M([1, 1], 4, "solid", ["shrine", "temple", "street", "waystation"], { free: ["religious"] }), 2, [
  ["a small roadside shrine, a carved stone niche with a candle and flowers", "A square stone top, a little niche at the front edge, wildflowers."],
  ["a wooden roadside shrine post with a small gabled roof and offerings", "A tiny shingle roof, ribbons and a bowl of grain."],
  ["a moss-covered stone shrine with a carved sun disc and coins", "Weathered stone, a round sun carving, copper coins at its foot."],
]);
fam("prop.offering_bowl", M([1, 1], 1, "clear", ["shrine", "temple"], { free: ["religious"] }), 2, [
  ["a bronze offering bowl full of coins, grain and flower petals", "A wide shallow bowl on a short foot."],
  ["a stone offering basin with floating candles", "Water in a round stone basin, small candles floating."],
  ["a clay offering dish with bread, honey and a ribbon", "A round red clay dish, a small loaf."],
]);
fam("prop.font", M([1, 1], 3, "solid", ["temple", "shrine"], { free: ["religious", "water"] }), 2, [
  ["a carved stone font, a round basin of still water on a pedestal", "Pale stone with a carved rim, clear water."],
  ["an octagonal marble font with a carved lid set to one side", "Eight-sided white marble rim, water, a wooden lid."],
  ["a rough granite font with moss and rainwater", "A round grey basin, moss at the rim."],
]);
fam("prop.incense_burner", M([1, 1], 3, "post", ["temple", "shrine", "apothecary"], { free: ["religious", "decoration"] }), 3, [
  ["a bronze incense burner on three legs with a pierced lid", "A round pierced dome, a thin wisp kept to the lid only."],
  ["a hanging censer set down on a stone stand", "A brass censer with chains coiled beside it."],
  ["a clay incense bowl with sticks of incense", "A round red bowl of sand, three thin sticks."],
]);
fam("prop.prayer_mat", M([1, 1], 0, "clear", ["temple", "shrine"], { layer: "floor", free: ["religious", "floor"] }), 3, [
  ["a small square woven prayer mat with a simple border", "Red and cream weave with tassels at two corners; lying flat."],
  ["a round woven reed meditation mat with a cushion on it", "A coiled reed mat and a plump blue cushion."],
  ["a small square felt mat in green with a geometric border", "Soft green felt, cream border; lying flat."],
]);
fam("prop.votive_rack", M([2, 1], 4, "solid", ["temple", "shrine"], { free: ["religious", "light"] }), 2, [
  ["an iron rack of many small lit votive candles in rows", "Tiers of little flames in cups, wax drips."],
  ["an iron rack of votive candles, most burnt out", "Rows of cups, a few small flames, heavy wax drips."],
  ["a sand tray of thin lit tapers", "A long tray of pale sand, tapers at angles, small flames."],
]);
fam("prop.reliquary", M([1, 1], 3, "solid", ["temple"], { free: ["religious", "decoration"], wealth: ["wealthy"] }), 3, [
  ["a gold reliquary casket on a stone plinth", "A small house-shaped gilt box with gems, a stone base."],
  ["a silver reliquary with a crystal window", "A domed silver case, a clear crystal panel."],
  ["a carved ivory-coloured reliquary box on a cushion", "A small carved casket on red velvet."],
]);
fam("prop.idol", M([1, 1], 3, "solid", ["shrine", "temple"], { free: ["religious"] }), 3, [
  ["a carved wooden idol of a seated figure with offerings at its feet", "A rounded carved head and shoulders, bowls of grain around it."],
  ["a stone idol of a many-armed figure on a small plinth", "Weathered grey stone, raised arms, a garland of flowers."],
  ["a small clay figurine shrine of a mother figure with flowers", "A round terracotta figure, a ring of petals."],
]);

// Keep and manor: wealthy pieces are their own ids, so a poor house never draws them.
fam("prop.table_wealthy", M([2, 1], 3, "solid", ["manor", "keep", "inn", "library"], { free: ["furniture"], wealth: ["wealthy"] }), 2, [
  ["a long carved dark wood dining table with a silver candelabra and a fruit bowl", "Polished walnut, gilded edge moulding, a runner of red cloth down the middle."],
  ["a long polished table laid with silver plates, crystal glasses and candles", "A white damask cloth, a silver centrepiece."],
  ["a long inlaid table with a marble top and gilt legs", "Veined white marble, a vase of roses at the centre."],
]);
fam("prop.bed_wealthy", M([1, 2], 3, "rough", ["manor", "keep", "inn"], { free: ["furniture"], wealth: ["wealthy"] }), 2, [
  ["a single carved wooden bed with white linen, a bolster and an embroidered coverlet", "Carved headboard at the top, deep green coverlet with gold thread trim."],
  ["a single bed with a padded red velvet headboard and silk sheets", "Headboard at the top, cream silk, a fur throw at the foot."],
  ["a single gilded bed with blue brocade covers", "Gilt headboard at the top, tasselled pillows."],
]);
fam("prop.four_poster_bed", M([2, 2], 7, "solid", ["manor", "keep", "inn"], { wealth: ["wealthy"], free: ["furniture"] }), 2, [
  ["a four-poster bed with a deep red canopy", "The canopy cloth with a carved post at each corner; pillows toward the top edge."],
  ["a large carved bed with white linen, many pillows and a fur throw", "The headboard along the top edge, a cream fur across the foot."],
  ["a four-poster bed with a dark green canopy and gold tassels", "The canopy fills the view, a post at each corner."],
]);
fam("prop.chest_treasure", M([1, 1], 2, "low", ["manor", "keep", "temple"], { free: ["container"], wealth: ["wealthy"] }), 2, [
  ["a wooden chest with its lid open, full of gold coins and a jewelled goblet", "Dark oak with iron bands, the lid raised, coins heaped and glinting."],
  ["a carved oak chest with brass fittings and a painted floral lid", "Carved scrolls along the lid's edge, red and green painted flowers."],
  ["an iron-bound chest spilling silver plate and pearls", "The lid ajar, silver cups, strings of pearls."],
]);
fam("prop.hearth_grand", M([2, 1], 5, "solid", ["manor", "keep"], { free: ["light", "hearth"], wealth: ["wealthy"] }), 2, [
  ["a grand carved marble fireplace with a roaring fire and brass fire irons", "Its back wall along the top edge, a carved mantel ledge; orange flames, no smoke."],
  ["a wide stone fireplace with a carved hood, a big log fire and iron dogs", "Its back wall along the top edge, heraldic carving without letters; no smoke."],
  ["a grand fireplace with a cold hearth, polished fender and a fire screen", "Its back wall along the top edge, a brass fender, a painted screen."],
]);
fam("prop.wardrobe", M([2, 1], 7, "tall", ["manor", "house", "inn", "keep"], { free: ["furniture", "storage"] }), 2, [
  ["a tall double wardrobe of dark oak", "Its moulded top board, the doors along the front edge, iron handles."],
  ["a tall painted wardrobe with carved panels", "Its top board, pale blue paint, flowers on the doors along the front edge."],
  ["a tall plain pine wardrobe with a hat box on top", "Its top board with a round box, the doors along the front edge."],
]);
fam("prop.dresser", M([2, 1], 4, "solid", ["manor", "house", "inn"], { free: ["furniture", "storage"] }), 2, [
  ["a chest of drawers with a basin, a jug and a small mirror on top", "A polished top, a porcelain jug and bowl, a hairbrush."],
  ["a vanity table with a round mirror, perfume bottles and a jewel box", "Gilded edges, cut-glass bottles and a lace runner."],
  ["a low dresser with folded linens and a candle", "A plain oak top, a stack of linen."],
]);
fam("prop.armchair", M([1, 1], 3, "solid", ["manor", "library", "inn", "house"], { free: ["furniture", "seating"], wealth: ["modest", "wealthy"] }), 2, [
  ["a deep leather armchair, cracked and comfortable", "Brown leather arms and a rounded back along the top edge, brass studs."],
  ["a wing-backed armchair in red velvet", "Tall wings along the top edge, carved feet."],
  ["a padded armchair upholstered in deep green", "Rounded arms, a buttoned back along the top edge."],
]);
fam("prop.settee", M([2, 1], 3, "solid", ["manor", "inn"], { free: ["furniture", "seating"], wealth: ["modest", "wealthy"] }), 2, [
  ["a cushioned settee upholstered in green brocade", "A carved back rail along the top edge, three cushions, rolled arms."],
  ["a long wooden settle with a high back and blue cushions", "A plank back along the top edge, a cushioned seat."],
  ["a settee in faded rose velvet with a shawl", "A curved back along the top edge, a knitted shawl."],
]);
fam("prop.side_table", M([1, 1], 3, "solid", ["manor", "house", "library", "inn"], { free: ["furniture"] }), 2, [
  ["a small side table with a candlestick and a vase of flowers", "A square polished top, a brass candlestick."],
  ["a small round side table with a wine decanter and two glasses", "A round marble top on a carved stand."],
  ["a small side table with a book and spectacles", "A square oak top, an open book."],
]);
fam("prop.harpsichord", M([2, 1], 4, "solid", ["manor"], { free: ["furniture", "decoration"], wealth: ["wealthy"] }), 3, [
  ["a harpsichord with its lid open showing strings and keys", "A wing-shaped case, painted lid interior with flowers, keys along one edge."],
  ["a harpsichord with its lid closed and sheet music on it", "A glossy black case, music without notes or letters."],
  ["a small upright harp beside a stool", "A carved curved frame with strings, a cushioned stool."],
]);
fam("prop.rug_large", M([3, 2], 0, "clear", ["manor", "keep", "inn", "temple", "library", "house"], { layer: "floor", free: ["decoration", "floor"] }), 2, [
  ["a large rectangular woven carpet with a central medallion in deep red and indigo", "A broad border of repeating motifs, fringed short ends; lying flat, no floor around it."],
  ["a large rectangular woven rug in faded green and gold stripes", "Worn bands of colour, fringed short ends; lying flat."],
  ["a large bear-skin rug with the head at one end", "Thick dark brown fur, paws spread; lying flat."],
  ["a large rich carpet with a gold-and-crimson floral field", "An intricate border, a slight sheen; lying flat."],
]);
fam("prop.potted_plant", M([1, 1], 3, "low", ["manor", "house", "inn", "temple", "library"], { free: ["decoration"] }), 2, [
  ["a potted fern in a terracotta pot", "Fronds radiating from the centre over the round pot rim."],
  ["a small potted lemon tree in a glazed blue pot", "A round crown of glossy leaves and yellow lemons."],
  ["a clay pot of red geraniums", "Bright red flower heads and round leaves over the pot rim."],
]);
fam("prop.bathtub", M([1, 2], 3, "solid", ["manor", "inn", "house"], { free: ["furniture", "water"] }), 3, [
  ["a wooden bathtub of soapy water with a towel over the rim", "Iron-banded staves, foam on the water, a folded towel."],
  ["a copper bathtub with clawed feet and steaming water", "A gleaming oval copper rim, warm water."],
  ["an empty wooden bathtub with a bucket beside it", "Dry staves, a wooden bucket and a brush."],
]);
fam("prop.feast_table", M([3, 1], 3, "solid", ["keep", "manor", "inn"], { free: ["furniture"] }), 2, [
  ["a very long trestle feast table laden with roasts, bread, fruit and goblets", "A heavy oak top running left to right, platters down its length, candles."],
  ["a very long high table covered in a white cloth with silver plates", "A pristine cloth, candelabra, silver plates and goblets."],
  ["a very long table after a feast, with bones, spilled wine and guttered candles", "Oak top left to right, scattered leftovers."],
]);
fam("prop.ballista", M([2, 2], 5, "solid", ["keep", "barracks"], { free: ["weapons", "structure"] }), 3, [
  ["a large wooden ballista on a turntable, loaded with a bolt", "Two bow arms spread, a long bolt along the stock pointing to the top edge."],
  ["an unloaded ballista with its cord slack and bolts stacked beside it", "Bow arms spread, a pile of long bolts."],
  ["a weathered ballista with a canvas cover over its stock", "Grey timber, a tied canvas sheet."],
]);
fam("prop.catapult", M([2, 3], 8, "solid", ["keep", "barracks"], { free: ["weapons", "structure"] }), 3, [
  ["a wooden catapult with its throwing arm cocked", "A heavy frame of beams, the arm lying along the long axis, a stone in the cup."],
  ["a counterweight trebuchet arm lying along a frame", "A long beam along the long axis, a box of stones at one end."],
  ["a broken catapult with a snapped arm", "A frame of beams, the arm split, ropes trailing."],
]);
fam("prop.armour_display", M([2, 1], 6, "solid", ["keep", "manor"], { free: ["weapons", "decoration"], wealth: ["wealthy"] }), 3, [
  ["a pair of suits of plate armour on a low wooden plinth", "Two helmeted heads, pauldrons and halberds, polished steel."],
  ["a pair of suits of gilded parade armour with plumed helmets", "Two crests with red plumes, gold-trimmed plates."],
  ["a rack of crossed swords and a shield on a plinth", "Polished blades in a fan, a painted shield."],
]);

// House, cottage and farmhouse.
fam("prop.spinning_wheel", M([1, 1], 4, "solid", ["cottage", "farmhouse", "house", "workshop"], { free: ["tool", "craft:weaving"] }), 2, [
  ["a wooden spinning wheel with a basket of wool", "The large wheel rim as a band across the centre, a stool and a basket of fleece."],
  ["a spinning wheel with a half-full bobbin of red yarn", "The wheel, the treadle and a bobbin wound with red thread."],
  ["a painted spinning wheel with a distaff of flax", "A green-painted wheel, a bundle of pale flax."],
]);
fam("prop.butter_churn", M([1, 1], 3, "solid", ["farmhouse", "farm", "cottage"], { free: ["tool", "food"] }), 3, [
  ["a wooden butter churn with a plunger", "A tall barrel-shaped churn seen from above, the plunger handle sticking up."],
  ["a barrel churn on a stand with a crank", "A small barrel on its side, an iron crank."],
  ["a butter churn beside a crock of butter", "A churn top, a round crock of pale butter."],
]);
fam("prop.cradle", M([1, 1], 2, "low", ["house", "cottage", "farmhouse", "manor"], { free: ["furniture"] }), 3, [
  ["a wooden rocking cradle with a small quilt", "Curved rockers at the ends, a patchwork blanket inside."],
  ["a woven wicker cradle with a white blanket", "A basket cradle, a white wool cover."],
  ["a carved cradle with a hood", "A wooden hood at one end, a soft blue blanket."],
]);
fam("prop.washtub", M([1, 1], 2, "low", ["house", "cottage", "farmhouse", "inn", "street"], { free: ["container", "water"] }), 2, [
  ["a wooden washtub of soapy water with a washboard", "Iron-banded staves, white foam, a ribbed board leaning in."],
  ["a washtub with wrung laundry draped over its rim", "Grey water, twisted white linen hanging over the edge."],
  ["a tin washtub with a scrubbing brush and soap", "A round grey tin rim, clear water."],
]);
fam("prop.kitchen_counter", M([2, 1], 3, "solid", ["house", "farmhouse", "inn", "tavern", "manor", "bakery"], { free: ["furniture", "food"] }), 2, [
  ["a kitchen work counter with chopped vegetables, a cleaver and pans", "A scrubbed pine top, carrots and onions, a copper pan."],
  ["a kitchen counter with a plucked chicken, a mortar and herbs", "A worn top, a pale fowl, sprigs of thyme and a knife."],
  ["a kitchen counter with bowls of dough and a rolling pin", "A floury top, a wooden pin, eggs."],
]);
fam("prop.pot_shelf", M([1, 1], 4, "solid", ["house", "farmhouse", "inn", "tavern"], { free: ["storage"] }), 3, [
  ["a low wooden rack of copper pots and iron pans", "Shiny copper and black iron rims in rows."],
  ["a low rack of clay crocks and jars with cloth lids", "Round crock mouths tied with cloth."],
  ["a low rack of wooden bowls, ladles and a churn dasher", "Stacked bowls, hanging spoons."],
]);

// Smithy and crafts (craft tags are arda-ids' `craft:<key>`).
fam("prop.quench_trough", M([2, 1], 2, "low", ["smithy"], { free: ["water", "tool", "craft:smith"] }), 2, [
  ["a smith's quenching trough of dark water with tongs and a cooling blade", "A long iron-banded box, dark water, a glowing-tipped blade."],
  ["a stone quenching trough with an oily sheen and a horseshoe", "A long grey box, dark water, a horseshoe."],
  ["a half barrel quenching tub with a set of tongs", "A cut barrel, dark water, iron tongs over the rim."],
]);
fam("prop.metal_stock", M([1, 1], 2, "rough", ["smithy", "workshop"], { free: ["container", "craft:smith"] }), 3, [
  ["a pile of iron bar stock and scrap", "Square bars, rods, a few horseshoes and rusty scraps."],
  ["a stack of iron ingots", "Dark grey bars in a crossed stack."],
  ["a bin of horseshoes and nails", "A wooden box of curved shoes and loose nails."],
]);
fam("prop.potters_wheel", M([1, 1], 3, "solid", ["workshop", "market"], { free: ["tool", "craft:pottery"] }), 2, [
  ["a potter's kick wheel with a half-thrown clay pot", "A round wheel head with a wet grey pot, a seat at the front edge."],
  ["a potter's wheel with a tall vase and tools", "A wet clay vase, a sponge, a wire and a bowl of slip."],
  ["a potter's wheel with a lump of red clay", "A round head, a wet red mound, a bucket."],
]);
fam("prop.kiln", M([2, 2], 6, "solid", ["workshop"], { free: ["hearth", "craft:pottery"] }), 2, [
  ["a round brick pottery kiln with a domed top and a glowing firebox", "A brick dome seen from above with a vent at the centre; no smoke."],
  ["a square stone kiln with its door open and stacked pots inside", "A stone box, rows of pale fired pots visible through the opening."],
  ["a cold round kiln with cracked clay and a woodpile", "A dull brick dome, a stack of split logs."],
]);
fam("prop.pottery_stack", M([1, 1], 2, "rough", ["workshop", "market", "stall", "warehouse"], { free: ["container", "craft:pottery"] }), 2, [
  ["a stack of clay pots, jugs and bowls", "Round rims of terracotta pots nested together."],
  ["a cluster of large clay storage jars", "Three tall jars with round mouths."],
  ["a stack of glazed plates and bowls", "Green and brown glaze, stacked in towers."],
]);
fam("prop.cobbler_bench", M([2, 1], 3, "solid", ["workshop", "market"], { free: ["tool", "craft:cobbling"] }), 2, [
  ["a cobbler's low bench with lasts, shoes and a hammer", "Wooden shoe forms, leather soles, waxed thread and tacks."],
  ["a cobbler's bench with a row of finished boots", "Polished brown boots in a row, a tack box."],
  ["a cobbler's bench with worn shoes waiting for repair", "Scuffed shoes, new soles, a knife."],
]);
fam("prop.jeweller_bench", M([2, 1], 3, "solid", ["workshop"], { free: ["tool", "craft:jewellery"], wealth: ["modest", "wealthy"] }), 2, [
  ["a jeweller's bench with tiny tools, a lamp and gems on a cloth", "A curved cutout front, files and tweezers, sparkling stones."],
  ["a jeweller's bench with a small anvil, wire and rings", "Silver wire coils, a tiny anvil, rings in a dish."],
  ["a jeweller's bench with a crucible and moulds", "A small crucible, stone moulds, a blowpipe."],
]);
fam("prop.tinker_bench", M([2, 1], 3, "solid", ["workshop"], { free: ["tool", "craft:tinkering"] }), 2, [
  ["a tinker's bench of gears, springs, pots and tools", "Small brass gears, a dented kettle, pliers and wire."],
  ["a tinker's bench with a half-built clockwork box", "An open brass box of gears, tiny screwdrivers."],
  ["a tinker's bench of pans being mended and solder", "Tin pans, a soldering iron, scraps."],
]);
fam("prop.carving_bench", M([2, 1], 3, "solid", ["workshop"], { free: ["tool", "craft:woodcarving"] }), 3, [
  ["a woodcarver's bench with a half-carved figure and gouges", "A wooden figure in progress, curled shavings, chisels in a row."],
  ["a woodcarver's bench with carved bowls and spoons", "Pale wooden bowls, spoons and a knife."],
  ["a woodcarver's bench with a carved panel of leaves", "A flat panel of leaf carving, mallet and gouges."],
]);
fam("prop.glass_furnace", M([2, 2], 5, "solid", ["workshop"], { free: ["hearth", "craft:glassblowing"] }), 3, [
  ["a round glassblower's furnace with glowing openings and blowpipes", "A brick dome seen from above, three glowing ports, long iron pipes."],
  ["a square glass furnace with a glowing crucible mouth", "A brick block, one bright opening, a gather on a pipe."],
  ["a cooling glass furnace with racks of finished bottles", "A brick dome, dull ports, a shelf of green bottles."],
]);
fam("prop.glass_shelf", M([1, 1], 5, "solid", ["workshop", "market", "apothecary"], { free: ["storage", "craft:glassblowing"] }), 3, [
  ["a shelf of glass bottles, bowls and goblets in green and amber", "Translucent glass catching the light in rows."],
  ["a shelf of blue glass vases and beads", "Cobalt blue glass, strings of beads."],
  ["a shelf of clear glass flasks and lamps", "Clear glass with pale reflections."],
]);
fam("prop.mason_block", M([1, 1], 3, "solid", ["workshop", "temple", "keep"], { free: ["rock", "craft:masonry"] }), 2, [
  ["a half-dressed stone block with a mallet and chisels", "A pale limestone block, one face dressed smooth, chips around it."],
  ["a stack of dressed building stones", "Squared grey blocks in two layers."],
  ["a carved stone capital in progress on a wooden pallet", "A block with half-carved leaves."],
]);
fam("prop.dye_vat", M([1, 1], 3, "rough", ["workshop", "tannery"], { free: ["container", "craft:weaving"] }), 2, [
  ["a wooden dye vat of deep blue liquid with cloth soaking in it", "Iron-banded staves, indigo water, a pole."],
  ["a wooden dye vat of madder red liquid", "Deep red water, a twisted cloth over the rim."],
  ["a wooden dye vat of golden yellow liquid", "Bright yellow water, a skein of wool."],
]);
fam("prop.paint_table", M([2, 1], 3, "solid", ["workshop", "manor"], { free: ["tool", "craft:painting"] }), 3, [
  ["a painter's table of pigment pots, brushes and a palette", "Bright colour dabs, jars of brushes, a rag."],
  ["a painter's table with a grinding slab and pigment powders", "A stone slab, small heaps of coloured powder."],
  ["a painter's table with rolled canvases and frames", "Rolled canvas, bare wooden frames, nails."],
]);

// Dock and boathouse.
fam("prop.pier_post", M([1, 1], 4, "post", ["dock", "boathouse"], { free: ["structure", "coastal"] }), 2, [
  ["a single round wooden pier piling with a rope tied round it", "A round log top with rings, a frayed rope looped and knotted."],
  ["a cluster of three tarred wooden pilings lashed together", "Three dark round tops bound with rope."],
  ["a weathered piling top covered in barnacles and green weed", "A round grey log end, white barnacles, weed."],
]);
fam("prop.bollard", M([1, 1], 2, "post", ["dock", "boathouse"], { free: ["structure", "coastal"] }), 2, [
  ["an iron mooring bollard with a rope coiled around it", "A squat black iron post, a thick rope wound round."],
  ["a stone mooring bollard", "A short rounded grey stone post with a worn groove."],
  ["a wooden cleat post with a rope figure-eight", "A short square post, a cross cleat, a rope."],
]);
fam("prop.fishing_net", M([2, 2], 0, "rough", ["dock", "boathouse"], { layer: "floor", free: ["coastal"] }), 2, [
  ["a fishing net spread flat to dry with cork floats", "A loose brown mesh with a line of round floats along one side; lying flat."],
  ["a heaped fishing net with floats and a few fish caught in it", "A tangled pile of brown mesh, corks and silver fish."],
  ["a folded fishing net with glass floats", "Neat folds of netting, green glass floats in rope cages."],
]);
fam("prop.net_rack", M([2, 1], 6, "solid", ["dock", "boathouse"], { free: ["structure", "coastal"] }), 2, [
  ["a wooden drying rack with fishing nets hanging over it", "A long pole on posts running left to right, nets draped in folds."],
  ["a drying rack with a torn net and a mending needle", "A pole left to right, a net with a gap, a wooden needle."],
  ["a drying rack with nets and cork float strings", "A pole left to right, nets and lines of floats."],
]);
fam("prop.crab_pot", M([1, 1], 2, "rough", ["dock", "boathouse"], { free: ["container", "coastal"] }), 2, [
  ["a stack of wicker crab pots", "Round woven pots with funnel openings, rope tied."],
  ["a wooden lobster creel with a red crab inside", "A slatted half-round trap, netting, a crab."],
  ["a pile of painted wooden fishing buoys", "Round and bullet-shaped floats in red, white and blue, rope tails."],
]);
fam("prop.anchor", M([1, 1], 1, "rough", ["dock", "boathouse"], { free: ["coastal"] }), 2, [
  ["a large iron ship's anchor lying flat with chain", "The shank across the frame, curved flukes at one end, a ring and chain."],
  ["a small rusty grapnel anchor with a rope", "Four curved hooks, orange rust, coiled rope."],
  ["a stone anchor, a holed rock tied with rope", "A rounded grey stone with a hole, a knotted rope."],
]);
fam("prop.rope_coil", M([1, 1], 1, "rough", ["dock", "boathouse", "warehouse", "mine"], { free: ["container"] }), 2, [
  ["a big neat coil of thick hemp rope", "A flat spiral of rope, the loose end trailing."],
  ["a messy pile of tangled rope", "Loops and kinks of tan rope."],
  ["a coil of rope with an iron hook", "A spiral of rope, a hook on top."],
]);
fam("prop.canoe", M([1, 2], 2, "rough", ["dock", "boathouse", "lumber_camp"], { free: ["vehicle", "boat"], biome: ["temperate", "boreal", "wetland"] }), 2, [
  ["a wooden canoe with a paddle inside", "Pointed at both ends, bow at the top, ribbed hull; empty, no water around it."],
  ["a birch bark canoe", "Pale bark hull with dark stitched seams, bow at the top; no water around it."],
  ["a dugout canoe carved from one log", "A long hollowed trunk, bow at the top; no water around it."],
]);
fam("prop.sailboat", M([2, 4], 4, "solid", ["dock", "boathouse"], { free: ["vehicle", "boat", "coastal"] }), 2, [
  ["a small single-masted sailing boat with its sail furled", "Bow at the top, a wooden deck, a mast and boom with a furled cream sail; no water around it."],
  ["a small fishing boat with a red furled sail and nets", "Bow at the top, a mast, nets and baskets; no water around it."],
  ["a small sailing boat painted green with a blue furled sail", "Bow at the top, a tiller at the stern; no water around it."],
]);
fam("prop.boat_hull", M([2, 3], 4, "solid", ["boathouse", "dock"], { free: ["vehicle", "boat", "craft:carpentry"] }), 3, [
  ["an unfinished wooden boat hull on trestles, ribs showing", "Curved pale ribs, a few planks fastened, a keel along the long axis."],
  ["an upturned boat hull on trestles being tarred", "A dark keel along the long axis, a bucket of tar."],
  ["a planked boat hull nearly finished, pale new wood", "Smooth strakes, the bow toward the top."],
]);
fam("prop.gangplank", M([1, 2], 0, "clear", ["dock", "boathouse"], { layer: "floor", free: ["floor"] }), 2, [
  ["a wooden gangplank of four narrow planks with cleats, no rails", "Planks run top to bottom, small cross cleats for grip; square-cut ends."],
  ["a weathered grey gangplank of narrow planks with rope grips, no rails", "Planks run top to bottom, a rope loop at the top end; square-cut ends."],
  ["a single wide plank gangway with cross battens, no rails", "A broad board top to bottom, battens for grip; square-cut ends."],
]);
fam("prop.fish_rack", M([2, 1], 5, "solid", ["dock", "boathouse", "market"], { free: ["food", "coastal"] }), 3, [
  ["a wooden rack of fish split and hung to dry", "Poles running left to right, rows of pale dried fish."],
  ["a rack of smoked fish over a smouldering pit", "Golden-brown fish on poles left to right; no smoke."],
  ["a rack of drying squid and seaweed strips", "Pale squid and dark seaweed on lines left to right."],
]);
fam("prop.barge", M([2, 4], 3, "rough", ["dock", "warehouse"], { layer: "floor", free: ["vehicle", "boat"] }), 3, [
  ["a flat river barge loaded with sacks and barrels", "A long rectangular plank deck with low sides, cargo lashed under ropes; no water around it."],
  ["an empty flat river barge with a long sweep oar", "A long plank deck with low sides, an oar along one side; no water around it."],
  ["a flat river barge carrying timber logs", "A long plank deck, logs chained along it; no water around it."],
]);
fam("prop.beached_boat", M([1, 2], 3, "solid", ["dock", "boathouse"], { free: ["vehicle", "boat", "coastal", "ruin"] }), 3, [
  ["an old rowboat hull lying upside down", "The tarred keel along the long axis, curved planks, a few missing."],
  ["a wrecked rowboat half filled with sand", "Broken planks, sand in the bottom, bow at the top."],
  ["a rotting boat hull with grass growing through", "Grey ribs, gaps, green tufts."],
]);

// Outdoor dressing: fences, wagons, carts.
fam("prop.fence_picket", M([1, 1], 4, "solid", ["house", "cottage", "farm", "street", "farmhouse"], { free: ["barrier"] }), 2, [
  ["a 5-foot section of white picket fence running from left to right", "Pointed pickets in a row on a rail, a post at each end; straight across the frame's centre."],
  ["a 5-foot section of weathered grey picket fence running from left to right", "Pointed pickets, a few leaning, a post at each end; straight across the frame's centre."],
  ["a 5-foot section of picket fence with climbing roses, running from left to right", "Pointed pickets with pink roses twined along; straight across the frame's centre."],
]);
fam("prop.fence_wattle", M([1, 1], 4, "solid", ["farm", "cottage", "farmhouse", "street"], { free: ["barrier"] }), 2, [
  ["a 5-foot woven wattle hurdle fence panel running from left to right", "Hazel rods woven between upright stakes; straight across the frame's centre."],
  ["a 5-foot old wattle hurdle panel, sagging, running from left to right", "Grey woven rods, some loose, stakes at the ends; straight across the frame's centre."],
  ["a 5-foot wattle hurdle panel of fresh green withies, running from left to right", "Pale green woven rods, stakes at the ends; straight across the frame's centre."],
]);
fam("prop.wagon", M([2, 3], 8, "total", ["street", "market", "waystation", "farm", "warehouse"], { free: ["vehicle"] }), 2, [
  ["a covered wagon with a white canvas hood on hoops", "The hood's ribs across it, the driver's bench and shafts at the top edge, big wheels at the sides."],
  ["an open four-wheeled freight wagon loaded with crates and barrels", "Shafts at the top, lashed cargo filling the bed, four spoked wheels."],
  ["a painted travelling wagon with a curved roof and a stovepipe", "A green and red barrel roof, a little chimney, steps at the back edge."],
]);
fam("prop.handcart", M([1, 1], 3, "solid", ["street", "market", "farm", "mine", "stall"], { free: ["vehicle"] }), 2, [
  ["a small two-wheeled handcart with sacks", "Two handles toward the bottom, a box bed, wheels at the sides."],
  ["a small handcart of firewood", "Two handles, a box bed heaped with split logs."],
  ["a small handcart of vegetables under a cloth", "Two handles, a box bed, cabbages under a cloth."],
]);
fam("prop.carriage", M([2, 3], 8, "total", ["manor", "keep", "street", "inn"], { free: ["vehicle"], wealth: ["wealthy"] }), 3, [
  ["an enclosed black carriage with gilt trim and a coachman's seat", "A glossy curved roof, lamps at the corners, four wheels, shafts at the top."],
  ["an open carriage with red leather seats", "Two facing seats, a folded hood, four wheels, shafts at the top."],
  ["an enclosed dark green carriage with luggage on the roof", "A curved roof with strapped trunks, shafts at the top."],
]);
fam("prop.broken_cart", M([1, 2], 3, "low", ["street", "farm", "waystation"], { free: ["vehicle", "ruin"], wealth: ["poor"] }), 3, [
  ["a broken-down two-wheeled cart tipped on one side with a missing wheel", "A tilted plank bed, one spoked wheel, spilled sacks."],
  ["an abandoned cart with a broken axle and weeds", "A sagging bed, a cracked wheel, grass around."],
  ["a burnt-out cart frame", "Charred planks and blackened iron rims."],
]);

// Farm and field dressing.
fam("prop.haystack", M([2, 2], 10, "total", ["farm", "barn"], { free: ["hay"] }), 2, [
  ["a round haystack with a thatched conical top", "Concentric layers of golden hay rising to a peak at the centre, a rope net."],
  ["a long haystack with a ridged thatch top", "A rounded rectangular stack with its ridge across the middle."],
  ["a round haystack partly eaten into at one side", "A conical top, one side cut away showing dense hay."],
]);
fam("prop.stooks", M([1, 1], 4, "low", ["farm"], { free: ["hay"] }), 2, [
  ["a stook of wheat sheaves leaning together", "Bundled golden sheaves in a small tent shape, ears at the top centre."],
  ["two stooks of barley sheaves", "Pale gold bundles leaning in pairs."],
  ["a stook of oat sheaves, slightly toppled", "Green-gold bundles, one fallen."],
]);
fam("prop.scarecrow", M([1, 1], 6, "post", ["farm"], { free: ["structure"] }), 2, [
  ["a scarecrow on a post with outstretched arms", "A straw hat at the centre, sleeves of a ragged coat along the crossbar."],
  ["a scarecrow with a turnip head and a red scarf", "A round head at the centre, a red scarf, straw hands."],
  ["a leaning scarecrow with a crow on its arm", "A battered hat at the centre, a black crow on the crossbar."],
]);
fam("prop.beehive", M([1, 1], 3, "solid", ["farm", "farmhouse", "cottage", "temple"], { free: ["structure"] }), 3, [
  ["a woven straw beehive skep on a wooden stand", "A round coiled straw dome, a small entrance, a few bees."],
  ["a wooden box beehive with a sloped lid", "A square box, a little landing board."],
  ["a row of two straw skeps under a small plank shelter", "Two coiled domes, a plank roof edge."],
]);
fam("prop.water_pump", M([1, 1], 4, "post", ["street", "farm", "market"], { free: ["structure", "water"] }), 3, [
  ["a cast-iron hand water pump over a stone basin", "The pump's handle and spout over a square stone basin of water."],
  ["a wooden pump post with an iron handle and a bucket", "A square timber post, a bucket under the spout."],
  ["a green-painted iron pump with a trough", "A round pump top, a long stone trough."],
]);
fam("prop.fountain", M([2, 2], 5, "solid", ["street", "market", "manor", "temple"], { free: ["structure", "water", "decoration"] }), 2, [
  ["a round stone town fountain with a central column and water", "A wide stone basin, a carved column with a small bowl at the centre."],
  ["an octagonal fountain with a stone fish spouting water", "Eight-sided rim, clear water, a carved fish in the middle."],
  ["a dry, cracked old fountain with leaves in the basin", "A round stone basin, moss, no water."],
]);

// Graves, crypts and ruins.
fam("prop.tomb", M([1, 2], 4, "solid", ["temple", "shrine", "keep", "manor"], { free: ["religious", "grave"] }), 2, [
  ["a stone sarcophagus with a carved effigy of a knight on its lid", "A long grey box, the figure's head at the top, hands folded on a sword."],
  ["a plain stone tomb chest with a carved border", "A long weathered grey block, a vine border; no lettering."],
  ["an open stone sarcophagus with its lid pushed aside", "A dark hollow inside, the lid slab askew, cobwebs."],
]);
fam("prop.coffin", M([1, 2], 2, "low", ["temple", "shrine"], { free: ["religious", "grave"] }), 3, [
  ["a closed wooden coffin", "A six-sided pine box, the wider end at the top."],
  ["an open empty wooden coffin with its lid leaning beside it", "A six-sided box, a dark empty interior."],
  ["a closed coffin draped with a dark cloth and flowers", "A black cloth, white lilies."],
]);
fam("prop.crypt_entrance", M([2, 2], 6, "total", ["temple", "shrine"], { free: ["religious", "grave", "structure"] }), 3, [
  ["a small stone crypt entrance with a sloping slab roof and iron door", "The slab roof seen from above, steps down to a black iron door at the front edge."],
  ["a mossy crypt entrance with a broken gate", "A moss-covered slab roof, a twisted iron gate."],
  ["a crypt entrance flanked by two small stone urns", "A grey roof slab, urns at the front corners."],
]);
fam("prop.bone_pile", M([1, 1], 1, "rough", ["shrine", "mine"], { free: ["ruin", "grave"], culture: [] }), 3, [
  ["a scatter of old bones and a skull", "Bleached long bones and a cracked skull."],
  ["a heap of animal bones and antlers", "Pale gnawed bones, a deer skull with antlers."],
  ["a bleached animal skull with long horns and a few ribs", "Sun-white bone, a horned skull."],
]);
fam("prop.skeleton", M([1, 2], 0, "clear", ["mine", "keep"], { layer: "floor", free: ["ruin", "grave"], culture: [] }), 3, [
  ["a human skeleton lying on its back with a rusty sword", "Skull at the top, ribs and arms, a corroded blade beside it."],
  ["a skeleton curled on its side in rotted rags", "Skull at the top, scraps of brown cloth."],
  ["a skeleton in rusty mail with a dented helmet", "Skull in a helm at the top, corroded rings."],
]);
fam("prop.rubble_pile", M([1, 1], 2, "rough", ["street", "keep", "mine", "temple"], { free: ["ruin", "rock"] }), 2, [
  ["a pile of broken masonry rubble and dust", "Chunks of dressed grey stone, broken mortar, grit."],
  ["a pile of fallen bricks and plaster", "Red-brown bricks, white plaster chunks."],
  ["a heap of charred timbers and ash", "Black burnt beams, grey ash, a few embers gone cold."],
  ["a scatter of fallen roof slates and broken tiles", "Grey slates and red tile shards."],
]);
fam("prop.rubble_large", M([2, 2], 4, "solid", ["street", "keep", "temple"], { free: ["ruin", "rock"] }), 2, [
  ["a large mound of collapsed stonework with a broken beam", "Big dressed blocks, rubble and a snapped timber."],
  ["a large heap of fallen masonry overgrown with weeds and ivy", "Mossy blocks, green creepers between them."],
  ["a collapsed wall section of tumbled bricks", "Bricks in a fan from one side."],
]);
fam("prop.broken_column", M([1, 1], 6, "tall", ["temple", "keep", "manor"], { free: ["ruin"] }), 2, [
  ["a broken stone column stump with fluting", "A round fluted shaft snapped off, jagged top, a square base."],
  ["a broken column overgrown with ivy", "A round grey stump wrapped in ivy."],
  ["a cracked square pillar with a fallen capital beside it", "A square stump, a carved capital block lying next to it."],
]);
fam("prop.fallen_column", M([3, 1], 3, "solid", ["temple", "keep"], { free: ["ruin"] }), 3, [
  ["a fallen fluted stone column lying in three pieces", "Round drums in a line left to right, a capital at one end."],
  ["a fallen mossy column lying in the grass", "A long round shaft left to right, moss and lichen."],
  ["a fallen square pillar cracked in two", "A long square shaft left to right, a split in the middle."],
]);
fam("prop.toppled_statue", M([1, 2], 2, "low", ["temple", "keep", "manor", "street"], { free: ["ruin"] }), 3, [
  ["a toppled stone statue of a robed figure lying on its back", "The head toward the top, a broken arm beside it, weathered."],
  ["a broken statue head and torso lying on the ground", "A huge stone face looking up, a cracked shoulder."],
  ["a toppled armoured statue with moss", "A helmed head toward the top, a stone shield, moss."],
]);
fam("prop.ruined_arch", M([3, 1], 10, "solid", ["temple", "keep"], { free: ["ruin", "structure"] }), 3, [
  ["a ruined stone archway, its top seen from above", "The curved arch top running left to right between two square piers, a few stones missing."],
  ["a ruined archway with ivy over the top", "The arch top left to right, green ivy trailing."],
  ["a half-collapsed arch with one pier standing", "One square pier, the arch broken off at the middle."],
]);

// Standing stones and cairns.
fam("prop.standing_stone", M([1, 1], 10, "tall", ["shrine"], { free: ["religious", "rock"], culture: [], wealth: [] }), 2, [
  ["a tall standing stone, a weathered grey monolith", "A rough rectangular top, lichen patches, grass at its foot."],
  ["a tall standing stone carved with spirals", "Faint carved spirals, moss on one side."],
  ["a leaning standing stone with a hole through it", "A rounded grey stone tilted, a round hole near the top."],
  ["a squat standing stone from a stone circle", "A rounded blocky grey stone, lichen, worn smooth on top."],
]);
fam("prop.cairn", M([1, 1], 4, "solid", ["shrine", "waystation", "street"], { free: ["marker", "rock"], culture: [], wealth: [] }), 2, [
  ["a stone cairn, a conical pile of rounded stones", "Grey stones heaped to a point, a flat stone on top."],
  ["a low cairn with a small wooden marker stick", "A rough mound of stones, a stick with a ribbon."],
  ["a mossy old cairn half sunk in grass", "Lichen-covered stones, grass creeping up the sides."],
]);
fam("prop.dolmen", M([2, 2], 6, "total", ["shrine"], { free: ["religious", "rock", "ruin"], culture: [], wealth: [] }), 3, [
  ["a dolmen, a huge flat capstone resting on upright stones", "The broad grey capstone seen from above, the upright stones peeking at the edges."],
  ["a mossy dolmen with a tilted capstone", "A green-furred capstone sloping to one side."],
  ["a collapsed dolmen, its capstone fallen", "A flat slab lying beside two leaning stones."],
]);

// Arid household.
fam("prop.clay_oven", M([1, 1], 4, "solid", ["bakery", "house", "farmhouse"], with_(arid, { free: ["hearth"] })), 2, [
  ["a beehive adobe oven of smooth ochre clay", "A round domed top, a small arched mouth at the front edge, soot at the opening."],
  ["a round open-topped clay pot oven with glowing coals", "A round mouth with orange coals deep inside, a thick clay rim."],
  ["a cracked adobe oven with a flat stone door", "A round ochre dome, a slab leaning at the mouth."],
]);
fam("prop.water_jars", M([1, 1], 3, "low", ["house", "market", "stall", "waystation"], with_(arid, { free: ["container", "water"] })), 2, [
  ["a cluster of three large terracotta water jars", "Round open mouths, rounded shoulders, a dipper."],
  ["a tall clay water jar in a rope sling on a wooden stand", "A round jar mouth with a wooden lid, a rope cradle."],
  ["a row of glazed blue and green clay jars", "Round mouths with cloth covers tied on."],
]);
fam("prop.shade_canopy", M([2, 2], 8, "tall", ["market", "stall", "street", "waystation"], with_(arid, { free: ["structure"] })), 2, [
  ["a square cloth shade canopy on four poles, faded red and cream", "The taut cloth fills the view with a pole at each corner."],
  ["a striped awning stretched on ropes between poles", "Blue and white stripes, ropes to the corners."],
  ["a palm-frond shade roof on four poles", "Dry fronds laid across a pole frame."],
]);
fam("prop.floor_cushions", M([2, 1], 1, "rough", ["house", "inn", "tavern", "manor"], with_(arid, { free: ["furniture", "seating"] })), 3, [
  ["a row of plump floor cushions on a low woven mat", "Embroidered cushions in red, orange and indigo."],
  ["a low divan of cushions with a small brass tray table", "Bolsters and cushions, a round tray with a teapot."],
  ["a pair of floor cushions and a carved low tea table", "Two cushions, a carved low table with cups."],
]);

// ---------------------------------------------------------------------------------------------
// Culture sets (tier 3): their own ids with an empty culture list and a free ancestry tag.

const cul = (c, fp, h, block, fn, free, more = {}) => M(fp, h, block, fn, with_(c, { free, ...more }));

fam("prop.table_dwarf", cul(dwarf, [2, 1], 3, "solid", ["house", "inn", "tavern", "keep", "manor"], ["furniture"]), 3, [
  ["a long stone slab table on two carved granite legs", "Smooth grey stone with angular knotwork along the edge, iron tankards on it."],
  ["a long table of dark timber bound with iron bands", "Heavy square legs, riveted straps, a stone jug."],
  ["a long stone feasting table with a carved border of interlocking squares", "Granite top, a roast and drinking horns."],
]);
fam("prop.chest_dwarf", cul(dwarf, [1, 1], 2, "low", ["house", "keep", "mine", "smithy"], ["container"]), 3, [
  ["a dwarven chest of dark wood with heavy square iron fittings", "Angular knotwork on the lid, a big lock plate; no letters."],
  ["a small stone-cut strongbox with a bronze lid", "Dressed grey stone, a geometric bronze lid."],
  ["a dwarven tool chest with an open lid of hammers and chisels", "Iron-bound dark wood, tools in fitted slots."],
]);
fam("prop.bed_dwarf", cul(dwarf, [1, 2], 2, "rough", ["house", "keep", "barracks", "inn"], ["furniture"]), 3, [
  ["a dwarven bed of heavy dark timber with thick furs and a carved headboard", "Angular knotwork on the headboard at the top, grey and brown furs."],
  ["a stone bed alcove slab with wool blankets", "A grey stone slab, folded red wool, a fur pillow."],
  ["a low iron-framed dwarven bed with a quilted cover", "Riveted iron frame, a brown quilt, headboard at the top."],
]);
fam("prop.statue_dwarf", cul(dwarf, [1, 1], 8, "tall", ["keep", "temple", "street", "mine"], ["decoration", "religious"]), 3, [
  ["a stone statue of a bearded dwarf holding a hammer, on a square plinth", "A broad helmeted head with a braided beard, the hammer head, the plinth edge."],
  ["a stone statue of a dwarf with a war axe and shield", "A helm and braided beard, a round shield, the plinth edge."],
  ["a stone statue of a seated dwarf king with a crown", "A crowned head, a broad beard over the knees, the plinth edge."],
]);
fam("prop.forge_dwarf", cul(dwarf, [2, 1], 4, "solid", ["smithy", "keep", "mine"], ["light", "tool", "craft:smith"]), 3, [
  ["a dwarven stone forge with angular carved stonework and glowing coals", "Square dressed granite, geometric carving, a bed of orange coals and big bellows; no smoke."],
  ["a dwarven forge with a bronze hood rim and a rune-free geometric frieze", "Dark stone, white-hot coals, tongs on a rack; no smoke."],
  ["a dwarven double forge with two coal beds and an anvil between", "Grey stone, two glowing beds, a heavy anvil; no smoke."],
]);
fam("prop.throne_dwarf", cul(dwarf, [1, 1], 5, "solid", ["keep", "manor"], ["furniture", "seating", "decoration"], { wealth: ["wealthy"] }), 3, [
  ["a massive stone throne carved from one granite block with angular knotwork", "A high square back along the top edge, heavy arms, a fur over the seat."],
  ["a dark iron throne with gold geometric inlay", "A high back along the top edge, square arms, a red cushion."],
  ["a stone throne flanked by two carved hammers", "A high back along the top edge, crossed stone hammers on the arms."],
]);
fam("prop.altar_dwarf", cul(dwarf, [2, 1], 4, "solid", ["temple", "shrine", "keep"], ["religious", "light"]), 3, [
  ["a dwarven altar of dark granite with a carved anvil and braziers", "Angular knotwork along its edges, a small anvil carving and two coal bowls."],
  ["a dwarven altar with a hammer laid on a bronze cloth", "Grey stone, a ceremonial hammer, candles."],
  ["a dwarven altar of stacked dressed stone with an ember pit", "Square courses of stone, glowing embers in a sunken bowl."],
]);
fam("prop.brazier_dwarf", cul(dwarf, [1, 1], 3, "solid", ["keep", "smithy", "temple", "mine"], ["light"]), 3, [
  ["a square bronze brazier with geometric cut-outs", "A box of bronze with angular patterns, glowing coals."],
  ["a squat iron fire bowl on a stone pedestal", "A wide iron bowl, orange coals, a carved base."],
  ["a stone brazier carved as a stylised mountain", "A peaked stone bowl, red embers."],
]);

fam("prop.table_elf", cul(elf, [2, 1], 3, "solid", ["house", "inn", "manor", "library"], ["furniture"]), 3, [
  ["a long table of pale wood with flowing leaf-shaped edges", "Silvery birch grain, curving carved vines along the edges, a glass bowl of fruit."],
  ["a long table grown from living wood with moss in the grain", "A smooth pale top, roots curling at the legs, white flowers."],
  ["a long table inlaid with mother-of-pearl leaves", "Pale ash, a silver jug and two slender cups."],
]);
fam("prop.chair_elf", cul(elf, [1, 1], 3, "solid", ["house", "inn", "manor", "library"], ["furniture", "seating"]), 3, [
  ["an elven chair of woven living branches", "Curving pale branches form the seat and the back along the top edge; small leaves."],
  ["a slender carved chair with a back like a spread leaf", "Pale wood, a leaf-shaped back along the top edge, a green cushion."],
  ["a curved bench-chair of bent willow", "Smooth looping willow rods, a moss-green cushion."],
]);
fam("prop.bed_elf", cul(elf, [1, 2], 2, "rough", ["house", "manor", "inn"], ["furniture"]), 3, [
  ["an elven bed of curved pale wood with moss-green linen and leaf patterns", "A flowing headboard at the top like branches, silvery green covers."],
  ["an elven bed with a woven canopy of vines", "Pale frame, a leafy canopy, white linen."],
  ["a low elven bed of soft moss and white flowers on a wooden frame", "A curved frame, a mossy surface, a silver blanket."],
]);
fam("prop.statue_elf", cul(elf, [1, 1], 8, "tall", ["temple", "manor", "street", "shrine"], ["decoration", "religious"]), 3, [
  ["a slender statue of an elf with a bow, on a round plinth", "Pale marble, the head and shoulders, the bow's curve, ivy round the plinth."],
  ["a slender statue of an elf holding a lantern aloft", "Pale stone, a raised arm with a lantern, flowing hair."],
  ["a statue of an elf lady with folded wings of leaves", "White marble, leaf shapes around the shoulders."],
]);
fam("prop.altar_elf", cul(elf, [2, 1], 4, "solid", ["temple", "shrine"], ["religious", "light"]), 3, [
  ["an elven altar of a living tree stump with silver bowls and white flowers", "Pale bark, a moss-covered top, flowers and a silver dish."],
  ["an elven altar of pale stone wrapped in flowering vines", "A smooth slab, white blossoms, a crystal."],
  ["an elven moonstone altar with a shallow water bowl", "Milky stone, a round silver basin of water."],
]);
fam("prop.lantern_elf", cul(elf, [1, 1], 2, "rough", ["street", "house", "temple", "manor"], ["light"]), 3, [
  ["an elven glass lantern shaped like a closed flower bud, glowing softly", "Silvered petals of glass around a pale warm light, a slender stem foot."],
  ["an elven lantern of woven silver wire with a soft blue-white light", "A teardrop cage, the glow stays inside."],
  ["a slender elven lamp post top with a leaf-shaped shade", "A curling stem, a glowing leaf of glass."],
]);
fam("prop.throne_elf", cul(elf, [1, 1], 5, "solid", ["manor", "keep", "temple"], ["furniture", "seating", "decoration"], { wealth: ["wealthy"] }), 3, [
  ["a throne grown from living tree roots with a moss cushion", "Curving pale roots form the back along the top edge and the arms; leaves at the top."],
  ["a pale wooden throne with a back of carved branches and silver leaves", "Tall branching back along the top edge, a green cushion."],
  ["a throne of white stone with flowing vine carvings", "A high curved back along the top edge, a silver cushion."],
]);
fam("prop.bookshelf_elf", cul(elf, [2, 1], 7, "tall", ["library", "manor", "temple"], ["furniture", "storage"]), 3, [
  ["a tall curved bookshelf of pale wood shaped like a tree", "Its top board, branches as shelves, books and scrolls along the front edge."],
  ["a tall bookshelf of woven branches with scroll cases", "Its top board, pale tubes and slim books along the front edge."],
  ["a tall bookshelf of polished ash with leaf inlays", "Its top board, slim silver-spined books along the front edge."],
]);

fam("prop.table_orc", cul(orc, [2, 1], 3, "solid", ["house", "inn", "tavern", "keep", "barracks"], ["furniture"]), 3, [
  ["a crude long table of rough-hewn logs with bones and a hunting knife", "Split logs pegged together, a gnawed bone and a horn cup."],
  ["a long plank table on stumps with a haunch of meat and a cleaver", "Rough planks, a big joint of meat, a heavy cleaver."],
  ["a long scarred table with axe marks and spilled drink", "Dark rough boards, deep cuts, iron mugs."],
]);
fam("prop.chest_orc", cul(orc, [1, 1], 2, "low", ["house", "keep", "barracks"], ["container"]), 3, [
  ["a crude orcish chest of thick planks bound with rough iron and spikes", "Hammered iron straps, a few iron spikes at the corners, a hide over the lid."],
  ["a hide-covered chest tied with rope", "Shaggy brown hide over a box, rope knots."],
  ["an open crude chest of loot: dented cups and coins", "Rough planks, a jumble of tarnished metal."],
]);
fam("prop.bed_orc", cul(orc, [1, 2], 2, "rough", ["house", "barracks", "keep"], ["furniture"]), 3, [
  ["an orcish bed, a heap of furs and hides on a low log frame", "Shaggy pelts piled loosely, a bone charm hanging at the head."],
  ["a sleeping pit lined with straw and a wolf pelt", "A shallow ring of logs, straw and grey fur."],
  ["a low frame bed with a bear hide and a war club leaning on it", "A dark pelt, a heavy club."],
]);
fam("prop.throne_orc", cul(orc, [1, 1], 5, "solid", ["keep", "barracks"], ["furniture", "seating", "decoration"]), 3, [
  ["a crude throne of heavy timbers draped with hides and animal skulls", "A high back along the top edge of lashed beams, a bear hide over the seat."],
  ["a throne of piled stones and tusks", "Rough stones, two big tusks rising along the top edge, a fur."],
  ["an iron throne hammered from shields and blades", "Overlapping shields as a back along the top edge, a hide seat."],
]);
fam("prop.banner_orc", cul(orc, [1, 1], 8, "post", ["keep", "barracks", "shrine"], ["decoration"]), 3, [
  ["a ragged war banner of red-brown hide on a spear pole with animal skulls", "The pole top at the centre with a horned skull, the tattered hide below."],
  ["a war standard of a spear hung with tusks and feathers", "The pole top at the centre, tusks and black feathers."],
  ["a black cloth war banner with a red handprint emblem", "The pole top at the centre, torn black cloth."],
]);
fam("prop.brazier_orc", cul(orc, [1, 1], 3, "solid", ["keep", "barracks", "shrine"], ["light"]), 3, [
  ["an orcish brazier made from an upturned iron helmet on three spear shafts", "Coals glowing in the bowl, crude lashings on the legs."],
  ["a fire pit in a ring of horned skulls", "Glowing coals, bleached skulls around the rim."],
  ["a crude iron fire basket hung from a tripod of logs", "Orange flames in a cage, rough logs; no smoke."],
]);
fam("prop.tent_orc", cul(orc, [2, 2], 7, "total", ["barracks", "keep", "lumber_camp"], ["structure", "camp"]), 3, [
  ["an orcish hide tent stretched over curved poles", "A low dome of stitched brown hides, bone toggles, smoke hole at the centre."],
  ["a cone tent of patched hides with tusks at the peak", "Hide panels, a crown of tusks at the centre."],
  ["a hide yurt with a painted red band", "A round dome, a red ochre stripe, a door flap at one side."],
]);
fam("prop.weapon_rack_orc", cul(orc, [2, 1], 5, "solid", ["barracks", "keep"], ["storage", "weapons"]), 3, [
  ["a crude rack of jagged cleavers and spiked clubs", "Rough log frame lashed with hide, notched blades in a row."],
  ["a rack of heavy axes and bone-handled spears", "Lashed poles, big axe heads, bone grips."],
  ["a pile of crude weapons against a log frame", "Clubs, rusty blades and a shield of planks."],
]);
fam("prop.totem", cul(orc, [1, 1], 10, "post", ["shrine", "keep"], ["religious"]), 3, [
  ["a carved wooden totem pole with animal faces, painted red and black", "The top carving of a bird with spread wings at the centre."],
  ["a tall post hung with animal skulls, feathers and hide strips", "The post top with a horned skull at the centre, strips fanning out."],
  ["a totem of stacked stones painted with ochre handprints", "Round stones in a pillar, red marks."],
]);

// ---------------------------------------------------------------------------------------------
// Vegetation: alts of the 225's vegetation (tier 1). Trees keep their species.

alts("veg.tree_oak", 1, [
  ["the full canopy of a broad oak tree, slightly lopsided", "Lobed leaf clusters in big rounded masses of rich green, a heavier side; the trunk is hidden."],
  ["the canopy of an ancient oak with a gappy crown showing thick grey branches", "Lobed leaf masses broken by gaps, gnarled limbs visible between them."],
  ["the dense canopy of a young oak", "Tight round clusters of fresh green lobed leaves, a compact round outline."],
  ["the canopy of an oak in late summer, darker green with a few yellowing leaves", "Big rounded masses of deep green, scattered yellow and brown leaves."],
  ["the canopy of an oak with ivy and mistletoe in its branches", "Lobed leaf masses, darker ivy trails, pale round clumps of mistletoe."],
]);
alts("veg.tree_elm", 1, [
  ["the canopy of a tall elm with an irregular, billowing crown", "Dense serrated leaves in big lumpy masses, deep shade between."],
  ["the canopy of an elm with a narrower vase-shaped crown", "Branches fanning outward, serrated leaf clusters along them."],
  ["the canopy of an old elm with a broken limb", "Dense leaf masses with one side thinner, a bare grey branch."],
  ["the canopy of an elm with yellow-green young leaves", "Fresh pale serrated leaves in soft masses."],
]);
alts("veg.tree_birch", 1, [
  ["the airy canopy of a birch tree with drooping twigs", "Small bright green leaves on hanging twigs, white branch glimpses."],
  ["the canopy of a pair of birches growing close together", "Two overlapping light crowns, white branches between them."],
  ["the canopy of a young birch, small and open", "Sparse bright leaves, white stems clearly visible."],
  ["the canopy of a birch with catkins and yellow-green leaves", "Small leaves, hanging catkins, white branches."],
]);
alts("veg.tree_fruit", 1, [
  ["the canopy of a pear tree with green pears", "A rounded crown of glossy leaves, pale green pears among them."],
  ["the canopy of a cherry tree in white and pink blossom", "A rounded crown covered in blossom with a little fresh green."],
  ["the canopy of a plum tree with purple plums", "A rounded crown of dark green leaves and dusky purple fruit."],
  ["the canopy of an old gnarled apple tree with sparse fruit", "A wide, uneven crown, twisted branches, a few red apples."],
]);
alts("veg.tree_pine", 1, [
  ["the crown of a tall pine with a flat-topped, irregular crown", "Blue-green needle tufts in clumps, orange branch glimpses; the trunk is hidden."],
  ["the crown of a young pine, dense and star-shaped", "Tight tiers of long needles in a neat star outline."],
  ["the crown of a windswept pine leaning to one side", "Needle clusters streaming to one side, an uneven outline."],
  ["the crown of a pine with a few brown dead needle tufts", "Layered needle tiers, scattered rust-brown patches."],
  ["the crown of a pine with clusters of cones", "Long needle tufts, brown cones grouped at the branch tips."],
]);
alts("veg.tree_spruce", 1, [
  ["the crown of a tall dark spruce", "Short dark needles in dense concentric tiers forming a sharp star outline."],
  ["the crown of a young spruce, small and neat", "Tight bright green tiers, a perfect star shape."],
  ["the crown of an old spruce with drooping branches and hanging lichen", "Dark tiers, grey-green beard lichen on the branches."],
  ["the crown of a blue-green spruce", "Silvery blue-green needles in concentric tiers."],
  ["the crown of a spruce with cones at the tips", "Dark tiers with clusters of brown cones at the branch ends."],
]);
alts("veg.tree_willow", 1, [
  ["the canopy of a weeping willow with long trailing fronds", "Pale silvery green fronds cascading outward in a soft round outline."],
  ["the canopy of a pollarded willow with a knob of thin straight shoots", "Many thin upright shoots with narrow leaves from a central knuckle."],
  ["the canopy of a crack willow with a broad, rough crown", "Narrow grey-green leaves in rough masses, a few bare branches."],
  ["the canopy of a weeping willow, denser and darker green", "Long fronds in heavy curtains, darker leaves."],
]);
alts("veg.tree_dead", 1, [
  ["a dead oak with thick twisted bare branches", "Grey gnarled limbs spreading wide, no leaves."],
  ["a dead conifer, a bare spire of short branches", "A pale grey trunk top with short stubby branches radiating, no needles."],
  ["a lightning-struck dead tree, split and charred", "Bare branches, a black scorched split down the middle."],
  ["a dead tree hung with grey moss", "Bare branches draped with trailing grey moss."],
]);
alts("veg.tree_alder", 1, [
  ["the canopy of an alder with dangling catkins", "Rounded dark glossy leaves, brown catkins and small cones."],
  ["the canopy of a multi-stemmed alder clump", "Several overlapping round crowns of dark leaves."],
  ["the canopy of a young alder, small and oval", "A neat oval of dark green leaves."],
  ["the canopy of an alder leaning to one side", "A crown stretched to one side, dark leaves."],
]);
alts("veg.tree_stunted", 1, [
  ["a stunted wind-bent birch with a few leaves", "A sparse low crown of small leaves, twisted white branches."],
  ["a stunted gnarled hawthorn", "A dense low crown of tiny leaves and red berries, thorny twigs."],
  ["a stunted mountain pine, flattened and twisted", "A low mat of dark needles leaning one way."],
  ["a stunted rowan with orange berries", "A small sparse crown, feathery leaves and berry clusters."],
]);
alts("veg.bush", 1, [
  ["a round dark green holly bush", "Glossy spiky leaves, a few red berries."],
  ["a loose leafy hazel bush", "Broad soft leaves in open clumps, a few nut clusters."],
  ["a dense round box shrub", "Tiny dark leaves in a tight dome."],
  ["a scraggly bush with thin branches and few leaves", "An open tangle of twigs, small leaves."],
  ["a round elder bush with flat white flower heads", "Feathery leaves with creamy flat blossom clusters."],
]);
alts("veg.bush_flowering", 1, [
  ["a round wild rose bush with pink flowers", "Small leaves and five-petalled pink roses, thorny stems."],
  ["a round lilac bush with purple flower clusters", "Heart-shaped leaves, cone-shaped purple blossom."],
  ["a round broom bush with bright yellow flowers", "Fine green stems covered in yellow pea flowers."],
]);
alts("veg.juniper", 1, [
  ["a low spreading juniper mat", "Spiky blue-green foliage spread wide and flat."],
  ["a tall narrow juniper", "Dense grey-green spiky foliage in a tight column seen from above."],
  ["a juniper with many dusty blue berries", "Blue-green spikes heavy with pale blue berries."],
]);
alts("veg.fern", 1, [
  ["a clump of tall bracken", "Feathery triangular fronds in a dense overlapping clump."],
  ["a fern turning russet in autumn", "Fronds radiating from the centre, coppery brown."],
  ["a small cluster of young curled fern fiddleheads", "Tight green spirals and a few unfurled fronds."],
]);
alts("veg.heather", 1, [
  ["a low mound of white heather", "Fine foliage covered in tiny white flowers."],
  ["a mound of heather out of flower, brown and green", "Fine needle foliage in muted bronze and green."],
  ["a low mound of deep magenta bell heather", "Fine foliage with clusters of tiny bell flowers."],
]);
alts("veg.flower_patch", 1, [
  ["a small cluster of bluebells", "Nodding blue bell flowers over strap leaves."],
  ["a small cluster of yellow buttercups and daisies", "Glossy yellow cups and white daisies."],
  ["a small cluster of foxgloves", "Tall pink-purple flower spikes over soft leaves."],
  ["a small cluster of red poppies", "Red papery petals with dark centres."],
]);
alts("veg.tall_grass", 1, [
  ["a tuft of tall golden dry grass", "Long straw-coloured blades with feathery seed heads."],
  ["a tuft of tall grass with a few thistles", "Green blades, purple thistle heads."],
  ["a tuft of tall blue-green grass", "Arching blades with a blue sheen."],
]);
alts("veg.reeds", 1, [
  ["a dense clump of tall green reeds", "Slender stems and blade leaves in a spiky starburst, no seed heads."],
  ["a clump of reeds with dark purple plumes", "Stems and blades radiating, feathery purple-brown plumes."],
  ["a clump of winter reeds, dry and pale", "Straw-coloured broken stems and pale plumes."],
]);
alts("veg.cattail", 1, [
  ["a small clump of cattails with two seed heads", "Strap leaves radiating from the centre, two brown heads."],
  ["a clump of cattails with fluffy bursting heads", "Brown heads splitting into white fluff among the leaves."],
  ["a dense clump of cattails with many slim heads", "Crowded strap leaves, a cluster of dark brown heads."],
]);
alts("veg.lily_pads", 1, [
  ["a cluster of floating lily pads with pink lily flowers", "Round notched pads overlapping loosely, two pink flowers; no water painted around them."],
  ["a small cluster of floating lily pads without flowers", "Round notched green pads; no water painted around them."],
  ["a cluster of floating lily pads with yellow pond lilies", "Round pads, small yellow cup flowers; no water painted around them."],
]);
alts("veg.mushroom_ring", 1, [
  ["a fairy ring of red-capped white-spotted toadstools", "A loose circle of bright red spotted caps, open in the middle."],
  ["a fairy ring of tiny white mushrooms", "A broken circle of small pale caps, open in the middle."],
  ["a fairy ring of tall brown parasol mushrooms", "A loose circle of broad scaly caps, open in the middle."],
]);
alts("veg.fallen_log", 1, [
  ["a fallen birch log lying horizontally", "White papery bark with black marks, broken ends."],
  ["a rotting fallen log covered in moss and bracket fungi", "Soft green moss, shelf fungi along one side, crumbling ends."],
  ["a fallen pine log with broken branches", "Rough reddish bark, stubby branches, pale broken ends."],
  ["a hollow fallen log", "Rough bark, one open end showing a dark hollow."],
]);
alts("veg.stump", 1, [
  ["a mossy old tree stump", "A rotten round top furred with moss, roots spreading."],
  ["a freshly cut tree stump with sawdust", "A pale flat top with sharp rings, sawdust around it."],
  ["a broken tree stump with jagged splinters", "A shattered top with spikes of wood, bark around it."],
  ["a tree stump with a cluster of mushrooms", "A grey weathered top, brown mushrooms on its side."],
]);
alts("veg.boulder", 1, [
  ["a single large rounded boulder of pale sandstone", "Smooth buff stone with faint layers and a crack."],
  ["a single large mossy boulder", "A grey rounded stone half covered in thick moss."],
  ["a single large dark basalt boulder", "Blocky dark grey stone with sharp edges."],
  ["a split boulder in two halves", "A grey rounded stone cracked through the middle, a fern in the gap."],
]);
alts("veg.rock_small", 1, [
  ["a small rounded grey rock", "A smooth river stone with a pale band."],
  ["a small flat rock with lichen", "A flat slab with yellow and white lichen spots."],
  ["a small mossy rock", "A grey stone with a green moss cap."],
  ["a small pointed rock with a quartz vein", "An angular grey stone, a white vein across it."],
]);
alts("veg.rock_large", 1, [
  ["a large rounded granite rock with cracks", "A massive grey dome with speckled stone and lichen."],
  ["a large flat-topped slab rock", "A broad flat grey top with a few pools and cracks."],
  ["a large mossy rock with a fern growing on it", "Grey facets, thick moss, a fern in a crevice."],
]);
alts("veg.rock_outcrop", 1, [
  ["a big outcrop of pale limestone with fissures", "Grey-white blocky ledges cut by deep cracks, small plants in the cracks."],
  ["a big outcrop of dark slate tilted in layers", "Thin dark grey layers on edge, sharp ridges."],
  ["a big outcrop of grey granite with rounded domes", "Smooth rounded humps, lichen and a crack with heather."],
]);
alts("veg.stones", 1, [
  ["a small cluster of rounded river stones", "Smooth grey and buff pebbles and cobbles."],
  ["a small cluster of angular grey stones with moss", "Sharp-edged stones, moss on their tops."],
  ["a small cluster of flat stepping stones", "Flat grey stones close together."],
]);
alts("veg.scree_patch", 1, [
  ["a loose patch of pale grey limestone scree", "Angular pale fragments, dense in the middle and scattered at the edges."],
  ["a loose patch of dark slate scree", "Thin flat dark shards, thinning at the edges."],
  ["a loose patch of mixed scree with a few tufts of grass", "Grey fragments, small green tufts, thinning at the edges."],
]);

// ---------------------------------------------------------------------------------------------
// New vegetation by biome (tier 2, rarer pieces tier 3).

// Temperate.
fam("veg.tree_beech", tree([3, 3], 35, ["temperate"], ["forest", "broadleaf"]), 2, [
  ["the full canopy of a beech tree", "Layered sprays of oval glossy leaves in flat tiers, a smooth round outline; the trunk is hidden."],
  ["the canopy of a beech with a broad spreading crown", "Wide flat tiers of oval leaves, deep shade between."],
  ["the canopy of a young beech, small and dense", "Tight tiers of bright green oval leaves."],
  ["the canopy of an old beech with a gap and a broken limb", "Layered oval leaves, a grey branch in a gap."],
]);
fam("veg.tree_ash", tree([3, 3], 35, ["temperate"], ["forest", "broadleaf"]), 2, [
  ["the full canopy of an ash tree with feathery compound leaves", "Open, airy sprays of paired leaflets, light green; the trunk is hidden."],
  ["the canopy of an ash with bunches of seed keys", "Feathery leaves and hanging brown seed clusters."],
  ["the canopy of a thin, open ash", "Sparse feathery sprays, grey branches visible."],
  ["the canopy of a broad ash with darker leaves", "Dense feathery sprays in heavy masses."],
]);
fam("veg.tree_maple", tree([2, 2], 30, ["temperate"], ["forest", "broadleaf"]), 2, [
  ["the full canopy of a maple tree", "Clusters of five-pointed leaves in rounded masses of mid green."],
  ["the canopy of a maple with winged seed clusters", "Green five-pointed leaves, pale paired seeds."],
  ["the canopy of a young maple with a neat round crown", "Bright five-pointed leaves in a tidy dome."],
  ["the canopy of a field maple with small lobed leaves", "Small dark leaves in dense clumps."],
]);
fam("veg.tree_autumn", tree([3, 3], 30, ["temperate"], ["forest", "broadleaf", "autumn"]), 3, [
  ["the canopy of a broadleaf tree in full autumn colour, red and orange", "Rounded leaf masses in scarlet, orange and gold."],
  ["the canopy of an oak in autumn, russet and brown", "Lobed leaves in copper and brown."],
  ["the canopy of a beech in autumn, copper and orange", "Flat tiers of oval leaves in copper and russet."],
  ["the canopy of a maple in autumn, bright red", "Five-pointed leaves in scarlet and crimson."],
]);
fam("veg.sapling", plant([1, 1], 6, "rough", ["temperate"], ["tree", "forest"]), 2, [
  ["a young oak sapling", "A small open crown of a few lobed leaves on thin twigs."],
  ["a young birch sapling", "A few small bright leaves on a thin white stem."],
  ["a young pine sapling", "A small star of fresh needles."],
]);
fam("veg.bramble", plant([1, 1], 3, "low", ["temperate"], ["bush", "undergrowth"]), 2, [
  ["a tangled bramble patch", "Arching thorny stems, three-lobed leaves, a few blackberries."],
  ["a bramble patch with white flowers", "Thorny canes, white five-petalled flowers."],
  ["a dense bramble thicket with ripe blackberries", "Dark glossy berries among thorny stems."],
]);
fam("veg.nettles", plant([1, 1], 3, "rough", ["temperate"], ["undergrowth"]), 2, [
  ["a patch of stinging nettles", "Upright stems with toothed heart-shaped leaves in pairs, dark green."],
  ["a patch of nettles and dock leaves", "Toothed nettle leaves mixed with broad dock leaves."],
  ["a patch of flowering nettles with hanging green tassels", "Toothed leaves, drooping flower strings."],
]);
fam("veg.thistle", plant([1, 1], 3, "rough", ["temperate"], ["undergrowth", "meadow"]), 3, [
  ["a spiky thistle plant with purple flower heads", "A rosette of spiny leaves, three purple tufts."],
  ["a thistle gone to seed with white fluff", "Spiny grey-green leaves, white downy seed heads."],
  ["a large spear thistle with many buds", "A spiny rosette, green buds and one purple flower."],
]);
fam("veg.mushrooms", plant([1, 1], 0, "clear", ["temperate", "boreal"], ["forest", "fungus"]), 2, [
  ["a small cluster of brown woodland mushrooms", "A few round brown caps of different sizes, close together."],
  ["a small cluster of tall pale ink-cap mushrooms", "Shaggy white caps."],
  ["a small cluster of orange chanterelle mushrooms", "Frilly golden-orange funnel caps."],
]);

// Arid.
fam("veg.cactus_column", plant([1, 1], 10, "solid", [], ["arid", "cactus"], { culture: [] }), 2, [
  ["a tall column cactus with three upturned arms", "Ribbed green stems seen from above as a star of round tops, fine spines."],
  ["a single tall ribbed column cactus", "One round ribbed top with radiating spines and a few white flowers."],
  ["a cluster of five slim column cacti", "Round ribbed tops close together, pale spines."],
  ["a dying column cactus, grey and woody", "A ribbed grey top, the woody skeleton showing."],
]);
fam("veg.cactus_paddle", plant([1, 1], 4, "low", [], ["arid", "cactus"], { culture: [] }), 2, [
  ["a prickly pear cactus of flat oval pads", "Overlapping blue-green pads, spine dots, red fruits on the edges."],
  ["a prickly pear cactus with yellow flowers", "Flat oval pads, bright yellow cup flowers."],
  ["a sprawling prickly pear clump", "Many pads spreading wide, a few dry yellowing ones."],
]);
fam("veg.cactus_barrel", plant([1, 1], 2, "low", [], ["arid", "cactus"], { culture: [] }), 2, [
  ["a round barrel cactus", "A ribbed green ball with golden curved spines, a crown of red flowers."],
  ["a cluster of small round cacti", "Five ribbed green balls of different sizes."],
  ["a round barrel cactus with yellow fruit", "A ribbed ball, a ring of yellow fruit on top."],
]);
fam("veg.agave", plant([1, 1], 3, "low", [], ["arid"], { culture: [] }), 2, [
  ["an agave plant", "A rosette of thick spiky blue-grey leaves radiating from the centre."],
  ["an agave with a tall flower stalk", "A spiky blue-grey rosette, a thick stalk rising at the centre."],
  ["a cluster of small aloe plants", "Fleshy toothed green leaves in small rosettes."],
]);
fam("veg.sagebrush", plant([1, 1], 3, "rough", [], ["arid", "bush"], { culture: [] }), 2, [
  ["a silvery sagebrush shrub", "Small grey-green leaves in soft rounded clumps."],
  ["a dry desert scrub bush", "A tangle of twiggy stems, tiny olive leaves."],
  ["a creosote bush with tiny yellow flowers", "Small dark olive leaves, yellow specks."],
]);
fam("veg.thornbush", plant([1, 1], 5, "low", [], ["arid", "bush"], { culture: [] }), 2, [
  ["a thorny acacia bush", "A flat-topped tangle of thorny twigs, tiny feathery leaves."],
  ["a dense dry thornbush", "Grey thorny twigs with a few small leaves."],
  ["a thornbush with small yellow puffball flowers", "Thorny branches, fluffy yellow round flowers."],
]);
fam("veg.tumbleweed", plant([1, 1], 3, "clear", [], ["arid"], { culture: [] }), 3, [
  ["a dry tumbleweed ball", "A round tangle of pale brittle stems."],
  ["a dead dry bush, brittle and grey", "Bare twisted grey twigs."],
  ["a flattened tumbleweed caught on a stone", "A loose pale tangle, a small grey stone."],
]);
fam("veg.yucca", plant([1, 1], 5, "low", [], ["arid"], { culture: [] }), 3, [
  ["a yucca plant", "A starburst of stiff sword-shaped green leaves, a cream flower spike."],
  ["a tree-like yucca with several spiky heads", "Several starbursts of stiff leaves on short branches."],
  ["a small yucca with dry brown lower leaves", "A green spiky starburst, a skirt of dead leaves."],
]);
fam("veg.dry_grass", plant([1, 1], 2, "rough", [], ["arid", "grass"], { culture: [] }), 2, [
  ["a tuft of dry desert bunchgrass", "Pale straw-coloured thin blades in a round clump."],
  ["a tuft of spiky dry grass with seed heads", "Wiry tan blades, a few feathery seeds."],
  ["a sparse clump of dry wiry grass and pebbles", "Grey-green wiry blades."],
]);
fam("veg.tree_palm", tree([2, 2], 30, [], ["arid", "palm"], { culture: [] }), 2, [
  ["the crown of a date palm", "Long feathery fronds radiating from the centre in a star, date clusters at the middle."],
  ["the crown of a tall palm with drooping fronds", "Arching feathery fronds in a loose star, a few dry brown ones."],
  ["the crowns of two palms growing together", "Two overlapping stars of fronds."],
  ["the crown of a young fan palm", "Round fan-shaped leaves radiating from the centre."],
]);
fam("veg.tree_acacia", tree([3, 3], 25, [], ["arid"], { culture: [] }), 2, [
  ["the flat-topped canopy of an acacia tree", "A wide flat umbrella of tiny feathery leaves, an even round outline."],
  ["the canopy of an acacia with yellow blossom", "A flat feathery crown dotted with yellow puffball flowers."],
  ["the sparse canopy of a dry acacia", "Thin feathery leaves, many thorny branches visible."],
  ["the lopsided canopy of a wind-shaped acacia", "A flat crown stretched to one side."],
]);
fam("veg.tree_olive", tree([2, 2], 20, [], ["arid", "orchard"], { culture: [] }), 2, [
  ["the canopy of an olive tree", "A rounded silvery grey-green crown of narrow leaves, small black olives."],
  ["the canopy of an old gnarled olive tree", "An uneven silvery crown, a twisted branch showing."],
  ["the canopy of a young olive tree", "A small neat silvery crown."],
  ["the canopy of an olive tree with green olives", "Silvery narrow leaves, green fruit."],
]);
fam("veg.desert_rock", plant([2, 2], 6, "total", [], ["arid", "rock"], { culture: [] }), 2, [
  ["a large wind-carved red sandstone rock", "Rounded layered rust-red stone with smooth hollows."],
  ["a large pale sandstone boulder with cracks", "Buff layered stone, sand in the cracks."],
  ["a cluster of rounded orange rocks", "Smooth warm orange stones piled together."],
]);

// Coastal.
fam("veg.dune_grass", plant([1, 1], 3, "rough", ["temperate"], ["coastal", "grass"]), 2, [
  ["a tuft of marram dune grass", "Tough grey-green rolled blades arching from the centre."],
  ["a tuft of dune grass with seed spikes", "Wiry blades and pale straw seed spikes."],
  ["a tuft of sea lyme grass", "Broad blue-grey blades arching outward."],
]);
fam("veg.seaweed", plant([1, 1], 0, "rough", ["temperate"], ["coastal", "water_plant"], { layer: "floor" }), 2, [
  ["a heap of washed-up brown kelp", "Glossy olive-brown fronds and stems tangled together."],
  ["a mat of bladderwrack seaweed", "Olive fronds with round air bladders, wet sheen."],
  ["a scatter of green sea lettuce and shells", "Bright green crinkled leaves, a few white shells."],
]);
fam("veg.driftwood", plant([2, 1], 2, "low", ["temperate"], ["coastal", "log"]), 2, [
  ["a long piece of bleached driftwood", "A smooth silver-grey branch with worn knots."],
  ["a twisted driftwood root", "A pale tangle of smooth roots."],
  ["a few pieces of driftwood and an old plank", "Grey smooth sticks and a nail-studded board."],
]);
fam("veg.sea_rock", plant([1, 1], 3, "solid", ["temperate"], ["coastal", "rock"]), 2, [
  ["a dark wet rock covered in barnacles and limpets", "A rounded black rock, white barnacle crust, cone limpets."],
  ["a rock with green seaweed and mussels", "Grey rock, blue-black mussel clusters, weed."],
  ["a cluster of dark sea-worn rocks", "Smooth black rocks with white streaks."],
]);
fam("veg.tide_pool", plant([1, 1], 0, "rough", ["temperate"], ["coastal", "water"], { layer: "floor" }), 3, [
  ["a small rock pool with anemones and a starfish", "A ring of dark rock around clear water, red anemones."],
  ["a small rock pool with weed and pebbles", "A rocky rim, green weed, small shells."],
  ["a small rock pool with a crab and limpets", "A rocky rim, clear water, a small red crab."],
]);

// Wetland.
fam("veg.sedge", plant([1, 1], 3, "rough", ["wetland", "temperate"], ["reeds", "water_plant"]), 2, [
  ["a clump of sedge", "Narrow arching grass-like leaves from a central tuft, brown flower spikes."],
  ["a clump of rushes", "Round smooth green stems radiating, small brown flower tufts."],
  ["a clump of flowering yellow iris", "Sword-like leaves and bright yellow flowers."],
]);
fam("veg.tussock", plant([1, 1], 2, "rough", ["wetland", "boreal"], ["grass"]), 2, [
  ["a tussock of tufted marsh grass", "A dense raised mound of fine pale grass."],
  ["a tussock of cotton grass with white tufts", "Thin leaves and fluffy white seed heads."],
  ["a sphagnum moss hummock", "A rounded cushion of red and green moss."],
]);
fam("veg.tree_mangrove", tree([3, 3], 20, ["wetland"], ["coastal", "riverside"]), 3, [
  ["the canopy of a mangrove tree", "A dense rounded crown of glossy oval leaves, arching roots peeking at the edges."],
  ["the canopy of a low spreading mangrove", "A wide flat crown of glossy leaves."],
  ["the canopy of a young mangrove", "A small round crown, prop roots at the edges."],
]);
fam("veg.tree_cypress", tree([2, 2], 40, ["wetland"], ["riverside"]), 2, [
  ["the canopy of a swamp cypress", "Feathery soft needles in a conical crown, grey moss strands."],
  ["the canopy of a swamp cypress in autumn, rusty orange", "Feathery rust-orange needles."],
  ["the canopy of an old swamp cypress with hanging moss", "A broad crown dripping with grey moss."],
]);
fam("veg.duckweed", plant([1, 1], 0, "clear", ["wetland", "temperate"], ["water_plant"], { layer: "floor" }), 2, [
  ["a floating mat of bright green duckweed", "Tiny round leaves packed together in a loose patch; no water painted around it."],
  ["a floating patch of water crowfoot with white flowers", "Thread-like leaves and small white flowers; no water around it."],
  ["a floating patch of pondweed with oval leaves", "Overlapping brown-green oval leaves; no water around it."],
]);
fam("veg.marsh_flowers", plant([1, 1], 2, "rough", ["wetland", "temperate"], ["flower"]), 3, [
  ["a clump of marsh marigolds", "Glossy round leaves and bright yellow cup flowers."],
  ["a clump of purple loosestrife", "Upright spikes of magenta flowers."],
  ["a clump of meadowsweet with creamy flower heads", "Frothy cream flowers over dark leaves."],
]);

// Boreal and alpine.
fam("veg.tree_larch", tree([2, 2], 40, ["boreal", "alpine"], ["forest", "conifer"]), 2, [
  ["the crown of a larch tree", "Soft tufts of bright green needles in layered tiers, a light open star shape."],
  ["the crown of a larch in autumn, golden yellow", "Tiers of golden needle tufts."],
  ["the crown of an old larch with lichen", "Sparse tiers, grey-green lichen on the branches."],
  ["the crown of a young larch", "A small neat star of soft needles."],
]);
fam("veg.tree_fir", tree([2, 2], 45, ["boreal", "alpine"], ["forest", "conifer"]), 2, [
  ["the crown of a silver fir", "Flat sprays of dark glossy needles in tiers, a pointed star outline."],
  ["the crown of a tall narrow fir", "Dense dark tiers in a slim star."],
  ["the crown of a fir with upright purple cones", "Dark tiers, upright cones at the top."],
  ["the crown of a young fir", "A neat small star of soft dark needles."],
]);
fam("veg.tree_spruce_snow", tree([2, 2], 45, ["boreal", "alpine"], ["forest", "conifer", "snow"]), 2, [
  ["the crown of a snow-laden spruce", "Concentric tiers of dark needles heavy with white snow, a star outline."],
  ["the crown of a young spruce dusted with snow", "Neat tiers, white snow on every tier."],
  ["the crown of a spruce with snow sliding off one side", "Dark tiers, thick snow on one half."],
  ["the crown of a frosted spruce with rime ice", "Silvery white frost on every needle."],
]);
fam("veg.tree_pine_snow", tree([2, 2], 40, ["boreal", "alpine"], ["forest", "conifer", "snow"]), 2, [
  ["the crown of a pine heavy with snow", "Needle clumps capped by white snow pillows."],
  ["the crown of a windswept pine with snow", "Needle tufts streaming to one side, snow on top."],
  ["the crown of a small snowy pine", "A small star of needles, white snow."],
]);
fam("veg.krummholz", plant([1, 1], 4, "low", ["alpine"], ["conifer", "bush"]), 3, [
  ["a low wind-flattened mountain pine mat", "Dense dark needle tufts spread flat and wide."],
  ["a stunted dwarf spruce clump", "A low dense cushion of dark needles."],
  ["a low mat of dwarf willow", "Tiny round leaves on creeping stems."],
]);
fam("veg.berry_bush", plant([1, 1], 2, "rough", ["boreal", "alpine", "temperate"], ["bush", "undergrowth"]), 2, [
  ["a low blueberry bush", "Small oval leaves, dusky blue berries."],
  ["a low lingonberry patch", "Glossy small leaves, bright red berries."],
  ["a low bilberry bush in autumn", "Red and purple small leaves, a few berries."],
]);
fam("veg.lichen_rock", plant([1, 1], 2, "solid", ["boreal", "alpine"], ["rock"]), 3, [
  ["a flat rock covered in grey and orange lichen", "Rosettes of orange, grey-green and black lichen."],
  ["a rock with pale reindeer moss around it", "A grey stone ringed by spongy white-green lichen."],
  ["a cracked rock with a cushion of moss campion", "Pink tiny flowers in a moss cushion in a crack."],
]);
fam("veg.rock_snow", plant([1, 1], 4, "solid", ["boreal", "alpine"], ["rock", "snow"]), 2, [
  ["a grey boulder capped with snow", "A rounded rock with a thick white snow cap, dark sides."],
  ["a cluster of rocks half buried in snow", "Dark stone tops poking through white snow."],
  ["a jagged rock with icicles and frost", "Sharp grey facets, frost and ice."],
]);
fam("veg.bush_snow", plant([1, 1], 3, "low", ["boreal", "alpine"], ["bush", "snow"]), 2, [
  ["a round bush covered in snow", "A white snow dome, dark twigs poking through."],
  ["a juniper bush dusted with snow", "Blue-green spikes under patchy snow."],
  ["a bare shrub with frost on its twigs", "A tangle of twigs rimed white."],
]);
fam("veg.alpine_flowers", plant([1, 1], 1, "rough", ["alpine"], ["flower", "meadow"]), 3, [
  ["a cushion of alpine flowers: gentians and edelweiss", "Deep blue trumpets and white woolly stars."],
  ["a cushion of alpine saxifrage", "Tight green rosettes with tiny white flowers."],
  ["a cushion of mountain avens", "White eight-petalled flowers over small dark leaves."],
]);

// ---------------------------------------------------------------------------------------------
// Underground (tier 4, arda-dungeon): what dungeons and caves need beyond the crypt and ruin sets.

fam("prop.stairs_down", M([1, 2], 0, "rough", [], { free: ["dungeon", "stairs", "stairs_down"], culture: [] }), 4, [
  ["a flight of stone steps descending into darkness, seen from directly above", "Grey stone treads, light at the bottom edge, each step darker towards the top edge, which ends in black shadow."],
  ["worn stone stairs going down between two low side walls", "Dished, chipped treads fading into a black opening at the top edge."],
  ["a narrow spiral of rough stone steps leading down into the dark", "Wedge-shaped treads curling towards a black hole near the top edge."],
]);
fam("prop.torch_sconce", M([1, 1], 0, "clear", [], { free: ["dungeon", "wall_light"], culture: [] }), 4, [
  ["an iron wall sconce holding a burning torch, seen from directly above", "A small iron bracket at the top edge of the frame, a wooden torch with a bright orange flame at its tip."],
  ["a rusty ring sconce with a smoking pitch torch", "Iron ring fixed at the top edge, a torch wrapped in dark rags, a small flame."],
  ["a wrought iron sconce shaped like a claw holding a torch", "Black iron claw at the top edge, a short torch with a yellow flame."],
]);
fam("veg.stalagmite", plant([1, 1], 6, "tall", [], ["cave", "stalagmite"]), 4, [
  ["a cluster of limestone stalagmites rising from a cave floor, seen from directly above", "Tapering pale cream and grey cones, the largest in the middle, each with a lighter wet tip."],
  ["a single thick stalagmite with a ring of smaller ones", "Rounded ridged pale stone, glistening tips."],
  ["a pair of stalagmites joined at the base, crusted with minerals", "Pale yellow-grey limestone with white mineral flow lines."],
]);
