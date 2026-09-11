# Contributing to rexafs

Changes to scientific behavior must include documentation that explains what
users are calculating and how to interpret the result. These requirements apply
to all project-authored documentation: READMEs, guides, research notes, examples,
Rust API comments, Python docstrings/type stubs, and TypeScript JSDoc/declarations.

## Documentation baseline

Write clear, grammatical English. Explain a concept before introducing its
abbreviation or equation. Use complete sentences and consistent scientific
terms. Preserve identifiers, source titles, quotations and provenance records
when their exact spelling matters; explain them in English where necessary.
Do not describe an unchecked statement as an established fact.

For each public operation, document its purpose, inputs, output, units,
recommended defaults, automatic values, errors and relevant side effects.
Explain whether arrays are copied, whether settings are copied, which cached
results are invalidated, and which prerequisite stages run automatically.
State which release implements the behavior. Python and TypeScript hover help
must convey the essentials without requiring the user to open another page.

When an equation helps explain an algorithm:

1. State the physical or numerical question it answers.
2. Define every symbol, index, dimension and unit, including normalization,
   Fourier sign/scaling, weights and fitted versus fixed quantities.
3. Explain the equation in words and describe the effect of its parameters.
4. State assumptions, approximations, validity limits and failure conditions.
5. Cite a verified primary paper, authoritative reference or official algorithm
   documentation next to the claim it supports. Use a DOI or stable source URL.
6. Link to the implementing function and explain any departure from the cited
   method. Identify project-specific derivations and empirical defaults as such.

A citation alone is not an explanation. A copied formula is not evidence that
rexafs implements that formula. Trace implementation-specific claims to the
current source and validate numerical examples. Do not invent references or
claim statistical confidence merely because an optimizer converged.

Keep the installation guides concise by linking to the
[processing theory](doc/processing-theory.md) and
[fitting statistics](doc/fitting-statistics.md). Repeat the short explanation
needed to use each API correctly; keep longer derivations in these shared guides.

## Scientific and historical records

State software versions, data provenance, hardware, timed boundaries and relevant
settings for measurements. Distinguish measured values from extrapolations,
synthetic truth from experimental references, and software agreement from
physical accuracy. Label prototypes and historical records so that they cannot
be mistaken for the current implementation. Retain raw evidence separately when
it is too large for source control; describe what evidence is publicly available.

Correct explanatory mistakes in historical teaching notes with a revision note.
Do not silently change archived input data, reference arrays or measured numbers.

## Review and validation

Review both the English and the scientific content. Check local links, equation
notation, cited sources and examples. For binding changes, build/install the
packages and run the runtime and language-server checks described in
[js-rexafs/test/README.md](js-rexafs/test/README.md). Follow the repository's
[development checks](doc/desktop-development.md) for code changes.

Automated checks can verify packaging, links, types and numerical examples.
They do not establish that prose is clear, a citation supports a claim, or an
assumption is physically appropriate; those require review.
