# D03 model review and contract attribution

This is an implementation audit of the frozen synthetic declarations, not an
independent human review. No reviewer or author per-case labor measurements
were supplied. Those fields remain null; measured replay wall time is not
person-hours. The 24 supplied cases remain exposed regression evidence.

The draft adapter consumes authoring inputs, not expected answers. Position
selectors retain their positions; source selectors resolve to the exact pinned
state script; token:first-input-id resolves to the canonical first input ID.
Long register reads retain their declared units, including named state tags.
The bounded-response trigger becomes the guard and its goal remains an assertion
on the single supplied trace, with the original two-action horizon. Schedule
identity binds the exact supplied transaction and environment at each step.
Unknown eventual/dynamic syntax is refused, never narrowed into supported syntax.

Eight accepted executions refute the author assertions. The deliberately weak
synthetic scripts do not enforce those assertions. Whether a contract ought to
enforce an assertion is unadjudicated. A wrong assertion can be faithfully
refuted by a correct contract; no row is attributed as a contract defect.
Eight controls hold on their supplied executions only. The inactive guard is
not a control. Missing/ambiguous bindings, wrong register type and overflow stay
unresolved; the short response trace supplies too few subsequent actions.
Eventuality and dynamic execution remain unsupported properties.

For the eight boundary rows, the machine report records the concrete first
failure: property expressiveness/meaning for missing/ambiguous/type and
unsupported syntax, operational cap for signed-i128 overflow, and action
representation for the supplied incomplete response horizon. These classifications
refer to the supplied material, not global impossibility. For the other 16 rows
(and the inactive guard's consequential interpretation), the first documented
obstacle to a consequential defect decision is missing independent review of
property meaning. Assumed external roots remain secondary: no declared
initialization was constructed, and no global unreachability claim follows.

The separate frozen transfer registration contains no cases. Its metric remains
null. The registration records the absence of defensible beyond-extraction
reference pairs and independent review. There are no transfer rows to invent or
relabel. This is a failed utility gate. D04 is not implemented or run here, and
no lifecycle or solver work is authorized by this report.
