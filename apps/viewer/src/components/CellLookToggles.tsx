/** The Cell tab's opt-in look toggles (goal 49): the world grade and the relief hand-off view. */
export function CellLookToggles({
  grade,
  onGrade,
  handoff,
  onHandoff,
  canHandoff,
}: {
  grade: boolean;
  onGrade: (on: boolean) => void;
  handoff: boolean;
  onHandoff: (on: boolean) => void;
  canHandoff: boolean;
}) {
  return (
    <>
      <label title="?world_grade=1 (goal 49, opt-in): pull the ground and water toward the world map's colours at this place">
        <input
          type="checkbox"
          aria-label="world grade"
          checked={grade}
          onChange={(e) => {
            onGrade(e.target.checked);
          }}
        />{" "}
        world grade
      </label>
      <label title="The World view's relief at the zoom where it hands over to the tactical map, beside this cell's tactical map">
        <input
          type="checkbox"
          aria-label="world hand-off"
          checked={handoff}
          disabled={!canHandoff}
          onChange={(e) => {
            onHandoff(e.target.checked);
          }}
        />{" "}
        world hand-off
      </label>
    </>
  );
}
