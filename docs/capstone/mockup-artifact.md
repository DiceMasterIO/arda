---
generated_date: 2026-08-24
generated_at_commit: 8d3c9d9
---

# Artifact — "Local area generator: the logic"

Provided verbatim by the user during the mockup interview (Q6),
2026-08-24. Seeds the mockup and, later, the logic stage.

---

# Local area generator: the logic

The project produces one local area for a top-down, grid-based D&D map: about 50 km across, divided into cells of roughly 100 m, each of which is itself a grid of D&D squares, generated once from a seed and a short description of the surrounding region. The result is a landscape in which every feature is where it is for a reason a geographer would accept: rivers run where water would go, forests stand where trees would grow, villages sit where people would build, and roads go where roads go. This document explains that reasoning, stage by stage, and nothing else.

## The premise

Most fantasy map generators draw each feature from its own noise. Mountains come from one noise field, forests from another, rivers are traced afterwards, towns are sprinkled where there is room. The parts look fine in isolation and wrong together, because nothing on the map caused anything else.

Real geography is a chain of consequences. Uplift and erosion fight, and the fight carves valleys. Water collects in those valleys and becomes rivers. Height and wind decide where it is cold and where it rains. Temperature, rain, slope and soil decide what grows. People settle where there is water, flat arable land, a hill to hold and a river to trade on. Roads connect the people along the paths of least effort, and the effort is set by the same terrain.

The generator follows that chain in order, and each stage is allowed to use only what the stages before it produced. Relief first, then water, climate, vegetation, settlement, land use, roads. That single rule is where the plausibility comes from. Everything below is a description of what each stage decides and on what grounds.

## Two grids, one world

The map has two levels that share one coordinate system. The coarse level is the area grid: cells of about 100 m, 512 to a side, 51 km across, about a day and a half on foot, the size of a barony or a hex-crawl region. The fine level is the D&D grid inside each cell: 5 ft squares, the ones miniatures stand on. A cell is a whole number of squares to a side so the two levels tile exactly; 64 squares (320 ft, 97.5 m) is the natural choice, and 66 gives an even 100 m. Every figure quoted per 100 m below holds for either. One cell is about one hectare, a unit the farmland arithmetic will use later.

The area level decides what each cell contains. The square level shows it. Everything in this document up to the last section is about the area level, because that is where the plausibility is decided: a village is put on a river terrace, a road is routed over a pass, a forest is placed on a north-facing slope, all as choices between cells. The squares inside a cell only have to draw faithfully what the cell says is there and join up with the squares of the neighbouring cells, and the last section describes that logic.

At the area level the map can hold valleys with side valleys, single villages and the fields around them, fords, bridges, passes, marshes, lakes of a few hectares, and roads that visibly follow terrain. It cannot hold buildings, the bends of a brook, or a switchback. Those exist only at the square level.

Two things follow from the scale. Plate tectonics is irrelevant: the mountains are not made inside a 50 km square, they are already there beyond its edge. And erosion becomes the central process, because at 100 m the shape of every valley is a direct product of how rivers cut and hillslopes fail.

## What the map is told about the world beyond its edges

The area is an excerpt from a larger region, so the generator has to be told what lies around it. That description is the only input besides the seed:

Which edge the land rises toward. That is the mountain side; water never leaves the map uphill through it, and the land near it is uplifted faster.

Which edge the sea lies along, if any. A landlocked area has no coast and drains off its low edges.

How high the land gets at the mountain edge, and how high the lowland sits above the sea.

Where the prevailing wind blows from, which decides which slopes are wet and which are in rain shadow.

The climate regime: temperate, boreal, mediterranean or tropical. It fixes the sea-level temperature, the typical rainfall, how high trees and permanent snow reach, and the temperature below which trees cannot grow at all.

Which rivers enter from outside, on which edge, at what point, and how large a catchment they already drain. A river that has gathered 800 km² of rain before it arrives is a real river from its first cell, and it carves a valley of its own.

How densely settled the region is, in people per square kilometre, and which edges the main roads should leave through, since a road network does not end at the edge of a map.

The area is computed slightly larger than what is shown and trimmed afterwards, so the visible edges behave like the middle of a landscape rather than like the walls of a box.

## Relief

The starting surface is deliberately crude: a gentle rise toward the mountain side that steepens near it, bent and roughened with noise so that no ridge runs in a straight line, and, if there is a coast, a ramp that drops the land below sea level along that edge. The ramp is pushed in and out by two noises, one long enough to make bays and headlands several kilometres across, one short enough to give the shore small irregularities. Wherever the ramp goes below sea level is sea.

None of that detail survives. What matters is what happens to it.

Two forces then work on the land for several hundred thousand years of simulated time. Uplift raises it, fastest near the mountain edge, slowly in the lowland. Rivers cut it down, and a river's cutting power grows with the amount of water it carries (its drainage area) and the steepness of its bed. Between the rivers, hillslopes creep downhill, and any slope steeper than about 35 degrees collapses until it is not. The sea is the floor: nothing erodes below sea level, and the sea cells themselves never move.

Out of that contest the whole shape of the map organises itself. Water finds the lowest neighbour of every cell and, by following those choices downhill, forms a tree of channels. Big channels carry more water, cut faster, and so become the valleys; small ones become their tributaries; the ridges are simply what is left. At equilibrium a channel's slope is set by the balance of uplift against cutting, and because cutting power grows with drainage area, headwater streams are steep and trunk rivers are gentle. A mountain stream draining a single square kilometre settles near a 9% slope, a river draining a hundred near 1%, and lowland channels are gentler still. That relationship, not any drawn shape, is what makes the valleys read correctly: V-shaped and steep at the top, broad and flat toward the coast.

The run is stopped before full equilibrium. The drainage network organises within the first quarter of the run; after that the peaks are still rising slowly, and a landscape still adjusting looks no less real than one that has finished.

Depressions that the uplift happens to create fill with water to their spill point and become lakes. They are not eroded from below toward a spill that sits above them; rivers simply pass through them. Rivers entering from outside are sealed in at their edge so they flow into the map instead of leaking back out beside their entry point.

## Water

Once the relief is final, water is traced through it one more time, now with real rainfall.

Every land cell drains to one of its eight neighbours, so the whole map is a tree with the sea and the low edges at its roots. Walking that tree gives each cell its drainage area (everything upstream of it) and its discharge (the rain that fell upstream, less the roughly half that evaporates or soaks in, plus whatever the entering rivers brought from beyond the edge).

A cell counts as a watercourse once it carries about 40 litres per second, which in a temperate climate means roughly three square kilometres of catchment above it. Below that, water still flows, but as ditches and seasonal trickles the map does not draw. Channel width grows with the square root of discharge and depth a little slower, following the measured relations for real rivers: a stream carrying one cubic metre a second is about four metres wide, a river carrying twenty-five is twenty.

Each watercourse has a Strahler order: headwater streams are first order, two first-order streams meeting make a second, two seconds make a third, while a smaller stream joining a larger one does not change the larger one's order. Order is a compact statement of how important a river is, and later stages use it for mouths, confluences and navigability.

The network is broken into segments, each running from a source or a junction downstream to the next junction, to the sea, into a lake, or off the edge of the map. Every segment knows which segment it feeds and how it ends.

Two derived quantities matter more than the rivers themselves for what follows. The height of a cell above the nearest watercourse downstream of it (found by following the drainage tree) says whether the cell is floodplain: under about a metre it is marsh, under two and a half it floods, and the terrace two to fifteen metres up is dry, close to water and flat, which is where villages have stood in every river valley in Europe. The second is a wetness index combining how much land drains through a cell with how flat it is; it finds seepage hollows and boggy flats even where no channel exists.

Lakes are recorded as objects with a surface level, a depth, and the cell where their water leaves.

## Climate

Temperature falls with height at six and a half degrees per kilometre, so a 1,500 m mountain above a 10 degree lowland has a mean just above freezing. Slopes facing the sun (south in the northern hemisphere) are up to a degree warmer than slopes facing away, with flat ground in between. That small difference shifts the treeline and the forest type by a visible band on opposite sides of a valley, which is one of the most recognisable features of real mountain country.

Rain is produced by carrying a parcel of moist air across the map in the wind direction. It arrives saturated at the upwind edge. Over flat ground it releases a small fraction of its moisture in every cell, so a plain dries only gently over 50 km. Wherever the ground rises, it releases much more, in proportion to the climb, and wherever the ground falls it releases little, so the windward face of a range is drenched and the far side is dry. Over sea and lakes it recharges. Over land it recovers a little of what it just dropped, standing in for evapotranspiration. The result is a rainfall map where a single range can carry eight times more rain on one flank than on the other, which is what real ranges do.

Rainfall alone does not say whether a place is wet. The generator compares it with the evaporation demand of the temperature: 600 mm is a saturated taiga at 2 degrees and a dry scrubland at 16. That ratio, the moisture index, is what vegetation and agriculture read.

## Vegetation and ground cover

Where trees can grow is a product of several independent limits, each scored from nothing to full. Temperature, relative to the regime's tree limit: full six degrees above it, nothing below. Moisture: nothing at an index of 0.3, full at 1. Slope: full up to 32 degrees, nothing at 44. Soil: convex ground, ridges and noses, has thin soil and loses up to a third; hollows keep it all. Aspect: in dry climates the shaded side of a valley holds more forest. And patchiness, a slow noise that gives real forests their clearings and dense cores. Along rivers in drier climates the low ground near water gains a gallery-forest bonus.

Where the combined potential is high the cell is forest, where it is middling it is scrub, where it is low it is grassland. Forest type follows temperature: conifer where cold, broadleaf where warm, mixed in between, with the thresholds set by the regime so the boreal preset is taiga throughout and the temperate one is oak below and pine above.

Some ground is decided by physics before vegetation gets a say, and these classes override in a fixed order. Flat land within a metre of a river, in a moist climate, is marsh; coastal ground barely above the sea is salt marsh. Flat floodplain a little higher is wet meadow unless forest has claimed it. Above the treeline, or where the mean temperature is within half a degree of the tree limit, the ground is alpine meadow. Slopes steeper than 38 degrees, or 28 degrees above the treeline, are bare rock. Gentle low ground touching the sea is beach; steep ground touching the sea is cliff. Above the snowline, or where the mean temperature is several degrees below the tree limit, there is permanent snow. Rivers more than 30 m wide occupy their cells as open water. A steep cell is rock whatever the trees would prefer; a coastal flat is beach even if it is wet.

## Where people settle

Every cell is scored as a place to live, from the surroundings it will actually have.

Water within a short walk counts most: a river, lake or the sea within 150 m is ideal, and the value fades to nothing beyond 750 m. Arable land within a kilometre counts nearly as much, because a village must feed itself from land it can reach before breakfast; arable means gentle slopes, warm enough and moist enough for crops, above flood level, and under grass, scrub, meadow or clearable forest. Flat ground to build on, warmth, and a hill with a view (prominence above the surrounding kilometre, worth full marks at 30 m) each add something. A sheltered coast, a bay or a river mouth rather than an open shore or a headland tip, is worth a great deal to a town. So are the natural nodes of travel: a confluence of two sizeable rivers, or a ford, a stretch of stream two to fourteen metres wide with gentle banks where a road can cross without a bridge.

Some cells are refused outright: open water, marsh, rock, snow, cliff, beach, anything steeper than 14 degrees, anything too cold to farm, and anything less than a metre and a half above the nearest watercourse. The last rule is the one people notice least and matters most: it keeps every settlement off the floodplain and puts it on the terrace above, which is where real ones are.

How many settlements there are follows from the land. The population is the region's density times the land area, and on the default map that is about forty thousand people. Three tenths of them live in towns, whose sizes follow the rank-size rule (the second town is half the first, the third a third), with as many towns as that rule allows while the smallest is still a town rather than a large village. Just under half live in villages of a few hundred, and the rest in hamlets of a dozen to eighty.

Placement is greedy, best site first, within each tier. The best-scoring cell takes the first town, a disk around it is closed to further towns (eight kilometres, a little under the distance medieval English law kept between markets), and the next best remaining cell takes the second. Towns weigh harbours and navigable rivers (fourth order or larger) far more heavily than villages do, which is why they end up at the estuary and in the bay. Villages keep a kilometre and a half from towns and two from each other, hamlets keep just under a kilometre from anything. A little randomness in the scores keeps the pattern from being mechanical, and a better site gets a larger settlement.

Each settlement then describes its own site: on a river, a lake or the coast; a harbour; at a confluence or a ford; on a hill; in forest, beside marsh, in the mountains. Those tags are for the referee, and they also decide the names. Real English place names encode the site (ford, mouth, haven, mere, bury, fen, hurst, strand), so a generated name takes its suffix from a matching tag and its stem from a syllable pattern, and a village called Dermouth sits at a river mouth because that is what it was named for.

## Land use around settlements

A settlement clears fields in proportion to its people, eight tenths of a hectare each, which at one hectare per cell means a village of three hundred farms about 240 cells. It takes the nearest arable cells first and prefers open ground to woodland, so forest is cleared last and only where fields have run out. A town of five thousand clears a plain two or three kilometres across; a hamlet clears a ring of a few dozen cells. Villages and hamlets also keep pasture beyond their fields on ground too steep or rough to plough. Towns keep no pasture; their meat comes from the villages.

The built-up footprint is sized from population at urban and village densities, and only on ground fit to build on. Buildings override fields, fields override pasture, and both override whatever grew there before.

## Roads

A road goes where the total effort of building and using it is least, and the effort depends on what the ground is. Flat grassland is the baseline. Fields, scrub and open woodland cost a little more, dense forest and alpine ground more again, marsh several times more, bare rock and snow many times, cliffs almost prohibitively. Slope multiplies everything, mildly at first and steeply beyond a 30% grade, so roads climb hills obliquely and follow valley floors and ridge crests. Crossing a watercourse costs in proportion to its width: fording a ten-metre stream is worth a kilometre of detour, a fifty-metre river four kilometres, so roads cross brooks freely and big rivers only when they must. Lakes and the sea cannot be crossed at all.

The single most important rule is that an existing road is cheap to travel. Once a road exists, any later route is drawn onto it, so the network grows as a tree with shared trunks instead of a tangle of parallel lanes. The same rule creates bridges as a matter of course: the first road to cross a river makes that crossing the cheapest place to cross, and every later road converges on it.

The network is built from the top of the hierarchy down, so that the lower tiers reuse the higher ones. Least-effort routes are found between every pair of towns and the cheapest set that connects them all becomes the trunk roads. Trunk roads are then pushed from the nearest town out to the map edges the region's roads leave through. Each village is connected to its nearest town, each hamlet to its nearest village, and because those routes fall onto existing roads wherever they can, a hamlet's track typically runs a few hundred metres before joining a road that already leads to the village.

Every place a road meets a watercourse is classified: small streams are bridged near settlements and forded elsewhere; middling rivers are bridged on trunk roads or near settlements and forded or ferried otherwise; wide rivers are ferried unless a trunk road and a nearby town justify a bridge. Where a road climbs over a ridge, the high point of its profile is recorded as a pass with its height and how far it stands above the valleys on either side.

## What the finished map knows

For every cell, which is also everything its block of squares needs: its height, whether it is land, sea or lake, its ground cover, its slope and which way it faces, its mean temperature, rainfall and moisture, how dense its forest would be before clearing, how much land and water drain through it, whether it is a watercourse and of what order and width, how far it stands above the nearest river, how wet it is, whether a road crosses it and of what class, and whether it is built on and by whom.

As objects: every river segment with its course, order, size and where it ends; every lake with its level, depth and outlet; every settlement with its name, tier, population, height and the features of its site; every road with its class, its two ends and its course; every crossing with its kind; every pass.

The same seed and the same regional description always produce the same area, so a map can be regenerated rather than kept, and changing how settled a region is does not change its terrain.

## How plausibility is checked

The area can be measured with the same statistics geographers apply to real terrain. The number of streams of each order should fall by a factor of three to five per order (Horton's ratio); a network cut too short by insufficient erosion gives a lower value, and running the relief longer raises it. The length of a river should grow with its drainage area to a power near 0.55 (Hack's law). Settlement populations should fall off as one over rank. Road length between two connected places divided by the straight-line distance should sit between 1.2 and 1.4 on open ground and rise in the mountains. The shares of forest, farmland and marsh should look like a real region of the same size and era, and farmland per inhabitant should be close to a hectare.

When a result looks wrong, the cause is usually a single dial. Too much mountain means too much relief or too fast an uplift. Mushy valleys mean too little erosion time. Cliffs everywhere mean the hillslope limit is too high. No rain shadow means the orographic effect is too weak. Villages all on the coast mean the harbour weight is too high or the interior valleys lack real rivers. Roads that cross rivers everywhere mean crossing is too cheap.

## Inside a cell: the D&D grid

A cell is a block of about 64 by 64 squares, and an ordinary battle map is a quarter of one. The block is not designed on its own terms; it is a faithful rendering of what its cell, and the eight cells around it, already say. The area level has done the thinking, so the square level has three jobs: put the right things in the block, put them in the right amounts, and make them continue into the neighbouring blocks.

What goes in the block comes from the cell. The ground cover chooses the kind of ground: forest floor, heath, ploughed strips, reed bed, scree, sand. Forest density says how many trees, and where the density changes between neighbouring cells the tree line thins across the block rather than stopping at its edge. Slope and aspect say how many 5 ft contour steps cross the block and in which direction, so a hillside block is a staircase of terraces running the right way and a valley-floor block is flat. Height above the river says whether there are pools, reeds and soft ground. A watercourse gives a band of water of the cell's width and depth, and its order says whether it is a brook to hop or a river to swim. A road gives a path band of a width set by its class, and a crossing record says whether the band meets the water on a bridge, through a ford or at a ferry landing. A pass puts the road on the block's highest ground. A settlement footprint fills the block with buildings at the density of its tier, the centre cell gets the church, the inn and the market, and the road through it becomes a street. Fields get their hedges or walls along the cell boundaries, because that is where field boundaries fall.

Continuity is the part that needs a rule rather than a drawing. Anything linear, a river, a road, a coastline, a contour, must leave one block at exactly the point where it enters the next, and the two blocks must agree on that point without looking at each other's finished squares. The point is fixed from the coarse data alone: a river enters a block from the side its upstream cell lies on and leaves toward the cell it drains to; a road enters from the previous road cell and leaves toward the next; the position along the shared edge is derived from the two cells' coordinates and the seed, so both sides compute the same answer. Inside the block the feature runs from its entry to its exit with a gentle curve, and its width is the cell's. Ground cover changes across a shared edge are drawn as a transition zone straddling the boundary rather than a hard line. With those rules, a party can walk from block to block, and a river or a road followed for twenty kilometres never breaks.

Wave Function Collapse is the natural way to fill a block. It fills a grid with tiles so that every pair of neighbours is a legal pairing, choosing the most constrained square first and propagating the consequences of each choice. It places whole tiles, so it cannot blend two of them; a transition from grass to water has to be a tile drawn as such. It is excellent at coherent local texture and has no concept of a river or a valley, which is why it has no part in the area level and is exactly right here: the cell's data becomes its constraints. Fixed tiles at the entry and exit points, a path that must connect them for each river and road, a count for trees and for buildings, and the adjacency rules of the tile set for everything in between. When a fill contradicts itself, the block is refilled with its border kept, and the block never sees anything larger than itself, so the failure modes of running it over a whole region never arise.

A block's content is a fixed consequence of the seed and the cell's coordinates, so it is the same every time it is drawn. That makes it a choice whether to draw every block in advance (a 512-cell area is about a billion squares, a gigabyte at a byte per square) or to draw each block the first time it is needed; the logic is identical either way.

## What is left out

Rivers run straighter than real ones on flat ground, because meanders and oxbows are not modelled; a mouth is a single cell rather than a delta or an estuary; springs exist only as the point where a channel begins, not as a geological feature; there are no glaciers above the snowline and no salt lakes in dry regimes; borders, mines, ruins and earlier settlement eras are not produced, although the road network and the town hierarchy give everything a border rule would need.

So World, Area, and tactical battle map 100m with grids.
