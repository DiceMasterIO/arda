// Spending and double-billing guards for generate.mjs. Pure planning (`spendPlan`, `guard`)
// plus the reconciliation of a POST whose reply may have been lost. Node 26, standard library only.

// Known per-image prices from Slopify's models.yaml (2026-09-26). Everything else is unknown
// there too; --price-per-image fills the gap.
export const knownPrices = {
  "google-image/gemini-3.1-flash-image": 0.101,
  "codex-image/*": 0,
};

export function priceOf(options) {
  const exact = knownPrices[`${options.provider}/${options.model}`] ?? knownPrices[`${options.provider}/*`];
  return options.pricePerImage ?? exact;
}

// Remakes the reviewer can order inside Slopify: each one is billed as another image.
export function remakesPerImage(options) {
  return options.review?.mode === "redo" ? options.review.retries : 0;
}

const cents = (n) => Math.round(n * 100) / 100;

// What this run can cost at most. `images` is what the run will submit; the cap on images
// (--max-images) lowers it; every image can be remade `remakesPerImage` times.
export function spendPlan(assets, options) {
  const images = Array.isArray(assets) ? assets.length : assets;
  const each = priceOf(options);
  const capped = options.maxImages === undefined ? images : Math.min(images, options.maxImages);
  const worstCaseImages = capped * (1 + remakesPerImage(options));
  const plan = {
    images,
    worstCaseImages,
    each,
    worstCaseCost: each === undefined ? undefined : cents(each * worstCaseImages),
  };
  if (images === 0) return plan;
  if (options.maxSpend !== undefined && each === undefined)
    plan.refusal =
      `--max-spend $${options.maxSpend} cannot be checked: ${options.provider}/${options.model} has no known price. ` +
      "Pass --price-per-image USD so the cap means something.";
  else if (options.maxSpend !== undefined && plan.worstCaseCost > options.maxSpend)
    plan.refusal =
      `the worst case is $${plan.worstCaseCost} (${worstCaseImages} image${worstCaseImages === 1 ? "" : "s"} × $${each}, ` +
      `with up to ${remakesPerImage(options)} reviewer remake(s) each), over --max-spend $${options.maxSpend}. ` +
      "Lower --limit, pass --max-images or --no-review, or raise --max-spend.";
  else if (options.yes && each === undefined && options.maxImages === undefined)
    plan.refusal =
      `${options.provider}/${options.model} has no known price, so the cost of --yes cannot be bounded. ` +
      "Pass --max-images N or --price-per-image USD.";
  return plan;
}

// The number of billable submissions this run may make, or Infinity.
export function submissionCap(options, each) {
  let cap = options.maxImages ?? Infinity;
  if (options.maxSpend !== undefined && each !== undefined && each > 0)
    cap = Math.min(cap, Math.floor(options.maxSpend / (each * (1 + remakesPerImage(options))) + 1e-9));
  return cap;
}

// A counter of billable submissions in this run: every new project, re-sent batch, re-posted
// failure and re-posted cancelled item takes from it before the request is made.
export function guard(options, each) {
  const cap = submissionCap(options, each);
  let used = 0;
  return {
    cap,
    get used() {
      return used;
    },
    get left() {
      return cap - used;
    },
    take(n = 1) {
      if (used + n > cap) return false;
      used += n;
      return true;
    },
    reason() {
      const parts = [];
      if (options.maxImages !== undefined) parts.push(`--max-images ${options.maxImages}`);
      if (options.maxSpend !== undefined) parts.push(`--max-spend $${options.maxSpend}`);
      return `${parts.join(" / ")} reached after ${used} submission${used === 1 ? "" : "s"}`;
    },
  };
}

// ---------------------------------------------------------------------------------------------
// An uncertain POST /api/projects

export class UnreadableListError extends Error {}

// Clock difference tolerated between this machine's intent record and Slopify's createdAt.
export const intentSkewMs = 5000;

// The newest project titled `id` created at or after the intent, or undefined when the list
// proves there is none. Throws UnreadableListError when the list cannot be read.
export async function findCreated(api, id, since, ignoreProjectId) {
  let projects;
  try {
    ({ projects = [] } = await api.get("/api/projects"));
  } catch (error) {
    throw new UnreadableListError(
      `cannot read Slopify's project list to check whether ${id} was already created (${error.message}); not posting it again.`,
    );
  }
  const floor = Date.parse(since) - intentSkewMs;
  const time = (p) => Date.parse(p.createdAt) || 0;
  return projects
    .filter((p) => p.title === id && p.id !== ignoreProjectId && !(Date.parse(p.createdAt) < floor))
    .sort((a, b) => time(b) - time(a) || (a.id < b.id ? 1 : -1))[0];
}

// Posts one asset as its own project. The intent is written before the request; a network
// error is settled by asking Slopify what exists, never by blindly posting again.
// Returns {project, adopted}. `save()` persists the state; `record` is the asset's finished record.
export async function postProject({ api, state, save, asset, body, record, now, log }) {
  const previous = state.assets[asset.id];
  const intent = { ...record, status: "posting", at: now() };
  state.assets[asset.id] = intent;
  save();
  for (let attempt = 1; ; attempt += 1) {
    try {
      const reply = await api.post("/api/projects", body);
      return { project: reply.project, adopted: false, reply };
    } catch (error) {
      if (error.network !== true) {
        // Slopify answered with a refusal: nothing was created.
        if (previous === undefined) delete state.assets[asset.id];
        else state.assets[asset.id] = previous;
        save();
        throw error;
      }
      const found = await findCreated(api, asset.id, intent.at, previous?.projectId);
      if (found !== undefined) {
        log(`ADOPTED ${asset.id}: the connection dropped but Slopify had created project ${found.id}; not posting it again`);
        return { project: found, adopted: true };
      }
      if (attempt >= 2) throw error;
      log(`${asset.id}: the connection dropped before Slopify created anything; posting it once more`);
    }
  }
}

// Leftover `posting` intents from a stopped run: adopt the project when one exists, release the
// asset for submission when the list proves none does. Returns {adopted: [ids], released: [ids]}.
export async function reconcileIntents(api, state, save, log) {
  const adopted = [];
  const released = [];
  for (const [id, record] of Object.entries(state.assets).filter(([, r]) => r.status === "posting")) {
    const found = await findCreated(api, id, record.at, undefined);
    if (found === undefined) {
      delete state.assets[id];
      released.push(id);
      log(`${id}: an earlier run stopped before posting it; Slopify has no such project, so it will be posted`);
    } else {
      Object.assign(record, { projectId: found.id, status: "submitted" });
      adopted.push(id);
      log(`ADOPTED ${id}: an earlier run's POST had created project ${found.id}; not posting it again`);
    }
    save();
  }
  return { adopted, released };
}
