--------------------------------------------------------------------------------
-- blobs
--------------------------------------------------------------------------------
CREATE TABLE blob
-- This table records basic metadata about blobs.
(
    -- An arbitrary integer identifier for the blob
    id integer primary key,

    -- The blob hash, computed a la Git, i.e., a hex digest of a fancy SHA-1 hash
    blob_id text unique not null,

    -- Size of the blob in bytes
    size integer not null,

    constraint valid_id check(
        length(blob_id) == 40 and not glob('*[^abcdefABCDEF1234567890]*', blob_id)
    ),

    constraint valid_size check(0 <= size)
) STRICT;

CREATE TABLE blob_mime_essence
-- This table records mime type metadata about blobs.
(
    -- The integer identifier of the blob
    blob_id integer primary key references blob(id),

    -- Guessed mime type of the blob
    mime_essence text not null
) STRICT;

CREATE TABLE blob_charset
-- This table records charset metadata about blobs.
(
    -- The integer identifier of the blob
    blob_id integer primary key references blob(id),

    -- Guessed charset encoding of the blob
    charset text not null
) STRICT;

CREATE TABLE blob_source_span
-- This table represents source span-based location information for ranges within blobs.
-- This allows you to look up line and column information given a (start byte, end byte) range.
(
    blob_id integer not null references blob(id),
    start_byte integer not null,
    end_byte integer not null,

    start_line integer not null,
    start_column integer not null,
    end_line integer not null,
    end_column integer not null,

    unique(blob_id, start_byte, end_byte),

    constraint valid_offsets check(0 <= start_byte and start_byte <= end_byte),

    constraint valid_span check(0 <= start_line
        and start_line <= end_line
        and 0 <= start_column
        and 0 <= end_column
    )
) STRICT;

CREATE TABLE blob_provenance
-- This table records the various ways in which blobs were encountered.
-- A blob can be encountered multiple ways when scanning; this table records all of them.
(
    -- The integer identifier of the blob
    blob_id integer not null references blob(id),

    -- The minified JSON-formatted provenance information
    -- XXX: deduplicate these values via another table?
    -- XXX: allow recursive representation of provenance values? I.e., structural decomposition and sharing, like `git repo` -> `commit` -> `blob path`?
    -- XXX: define special JSON object fields that will be handled specially by NP? E.g., `path`, `repo_path`, ...?
    provenance text not null,

    unique(blob_id, provenance),

    constraint payload_valid check(json_type(provenance) = 'object')
) STRICT;

--------------------------------------------------------------------------------
-- rules
--------------------------------------------------------------------------------
CREATE TABLE rule
-- This table records rules used for detection.
(
    -- An arbitrary integer identifier for the rule
    id integer primary key,

    -- The human-readable name of the rule
    name text not null,

    -- The textual identifier defined in the rule
    text_id text not null,

    -- A content-based identifier, defined as the hex-encoded sha1 hash of the pattern.
    structural_id text unique not null,

    -- The minified JSON serialization of the rule
    syntax text not null,

    constraint json_syntax_valid check(json_type(syntax) = 'object')
) STRICT;

--------------------------------------------------------------------------------
-- generic rules
--------------------------------------------------------------------------------
CREATE VIEW generic_rule_id (rule_id) AS
-- The set of IDs of rules that are categorized as `generic`
select id from rule
where exists (
    select 1 from json_each(syntax->>'categories')
    where value = 'generic'
);

--------------------------------------------------------------------------------
-- fuzzy rules
--------------------------------------------------------------------------------
CREATE VIEW fuzzy_rule_id (rule_id) AS
-- The set of IDs of rules that are categorized as `fuzzy`
select id from rule
where exists (
    select 1 from json_each(syntax->>'categories')
    where value = 'fuzzy'
);

--------------------------------------------------------------------------------
-- rule pattern length
--------------------------------------------------------------------------------
CREATE VIEW rule_pattern_length (rule_id, length) AS
-- The length of each rule's pattern in bytes
select id, length(syntax->>'pattern') length
from rule;

--------------------------------------------------------------------------------
-- snippets
--------------------------------------------------------------------------------
CREATE TABLE snippet
-- This table represents contextual snippets in a deduplicated way.
--
-- Deduplication of snippets reduces the size of large datastores 20-100x or more.
-- Keeping them in a separate table also makes it possible to update _just_ the
-- snippets of matches when scanning using a larger context window.
(
    -- An arbitrary integer identifier for the snippet
    id integer primary key,

    -- The snippet content
    snippet blob unique not null
) STRICT;

--------------------------------------------------------------------------------
-- findings
--------------------------------------------------------------------------------
CREATE TABLE finding
-- This table represents findings.
--
-- A finding is defined as a group of matches that have the same rule and groups.
-- Each finding is assigned a content-based identifier that is computed from
-- its rule and groups:
--
-- sha1_hex(rule structural identifier + '\0' + minified JSON array of base64-encoded groups)
(
    -- An arbitrary integer identifier for the match
    id integer primary key,

    finding_id text unique not null,

    -- The rule that produced this finding
    rule_id integer not null references rule(id),

    -- The capture groups, encoded as a minified JSON array of base64-encoded bytestrings
    groups text not null,

    constraint valid_id check(
        length(finding_id) == 40 and not glob('*[^abcdefABCDEF1234567890]*', finding_id)
    ),

    constraint valid_groups check(json_type(groups) = 'array'),

    unique(rule_id, groups)
) STRICT;

--------------------------------------------------------------------------------
-- matches
--------------------------------------------------------------------------------
CREATE TABLE match
-- This table represents the matches found from scanning.
--
-- See the `noseyparker::match_type::Match` type in noseyparker for correspondence.
(
    -- An arbitrary integer identifier for the match
    id integer primary key,

    -- The content-based unique identifier of the match
    -- sha1_hex(rule structural identifier + '\0' + hex blob id + '\0' + decimal start byte + '\0' + decimal end byte)
    structural_id text unique not null,

    -- The identifier of the finding this match belongs to
    finding_id integer not null references finding(id),

    -- The blob in which this match occurs
    blob_id integer not null references blob(id),

    -- The byte offset within the blob for the start of the match
    start_byte integer not null,

    -- The byte offset within the blob for the end of the match
    end_byte integer not null,

    -- the contextual snippet preceding the matching input
    before_snippet_id integer not null references snippet(id),

    -- the entire matching input
    matching_snippet_id integer not null references snippet(id),

    -- the contextual snippet trailing the matching input
    after_snippet_id integer not null references snippet(id),

    unique (
        blob_id,
        start_byte,
        end_byte,
        finding_id
    ),

    foreign key (blob_id, start_byte, end_byte)
        references blob_source_span(blob_id, start_byte, end_byte)
) STRICT;

CREATE INDEX match_finding_id_index on match(finding_id);

--------------------------------------------------------------------------------
-- Statuses
--------------------------------------------------------------------------------
CREATE TABLE match_status
-- This table records the accepted/rejected status of matches.
(
    -- The integer identifier of the match
    match_id integer primary key references match(id),

    -- The assigned status, either `accept` or `reject`
    status text not null,

    constraint status_valid check (status in ('accept', 'reject'))
) STRICT;

--------------------------------------------------------------------------------
-- Redundancies
--------------------------------------------------------------------------------
CREATE TABLE match_redundancy (
    -- The integer identifier of the match
    match_id integer not null references match(id),

    -- The integer identifier of the match that replaces `match_id`
    redundant_to integer not null references match(id),

    unique (match_id, redundant_to)
);

--------------------------------------------------------------------------------
-- Comments
--------------------------------------------------------------------------------
CREATE TABLE finding_comment
-- This table records ad-hoc comments assigned to findings.
(
    -- The integer identifier of the finding
    finding_id integer primary key references finding(id),

    -- The assigned comment, a non-empty string
    comment text not null,

    constraint comment_valid check (comment != '')
) STRICT;

CREATE TABLE match_comment
-- This table records ad-hoc comments assigned to matches.
(
    -- The integer identifier of the match
    match_id integer primary key references match(id),

    -- The assigned comment, a non-empty string
    comment text not null,

    constraint comment_valid check (comment != '')
) STRICT;

--------------------------------------------------------------------------------
-- Scores
--------------------------------------------------------------------------------
CREATE TABLE match_score
-- This table records a numeric score for matches.
(
    -- The integer identifier of the match
    match_id integer primary key references match(id),

    -- The numeric score in [0, 1]
    score real not null,

    constraint score_valid check (0.0 <= score and score <= 1.0)
) STRICT;

--------------------------------------------------------------------------------
-- Convenience Views
--------------------------------------------------------------------------------
CREATE VIEW match_denorm
-- A convenience view for matches in denormalized form rather than the
-- low-level datastore form that involves numerous indirections.
(
    id,
    structural_id,
    finding_id,

    blob_id,

    start_byte,
    end_byte,

    start_line,
    start_column,
    end_line,
    end_column,

    rule_name,
    rule_text_id,
    rule_structural_id,

    groups,

    before_snippet,
    matching_snippet,
    after_snippet,

    status,
    comment,
    score
) as
select
    m.id,
    m.structural_id,
    f.finding_id,

    b.blob_id,

    m.start_byte,
    m.end_byte,

    bss.start_line,
    bss.start_column,
    bss.end_line,
    bss.end_column,

    r.name,
    r.text_id,
    r.structural_id,

    f.groups,

    before_snippet.snippet,
    matching_snippet.snippet,
    after_snippet.snippet,

    match_status.status,
    match_comment.comment,
    match_score.score
from
    match m
    left outer join finding f on (m.finding_id = f.id)
    left outer join blob_source_span bss on (
        m.blob_id = bss.blob_id
            and
        m.start_byte = bss.start_byte
            and
        m.end_byte = bss.end_byte
    )
    left outer join blob b on (m.blob_id = b.id)
    left outer join rule r on (f.rule_id = r.id)
    left outer join snippet before_snippet on (m.before_snippet_id = before_snippet.id)
    left outer join snippet matching_snippet on (m.matching_snippet_id = matching_snippet.id)
    left outer join snippet after_snippet on (m.after_snippet_id = after_snippet.id)
    left outer join match_status on (m.id = match_status.match_id)
    left outer join match_comment on (m.id = match_comment.match_id)
    left outer join match_score on (m.id = match_score.match_id)
;

CREATE VIEW blob_denorm
-- A convenience view for blobs in denormalized form rather than the low-level
-- datastore form that involves numerous indirection.
(
    id,
    blob_id,
    size,
    mime_essence,
    charset
)
as
select
    b.id,
    b.blob_id,
    b.size,
    bm.mime_essence,
    bc.charset
from
    blob b
    left outer join blob_mime_essence bm on (b.id = bm.blob_id)
    left outer join blob_charset bc on (b.id = bc.blob_id)
;

CREATE VIEW blob_provenance_denorm
-- A convenience view for blob provenance in denormalized form rather than the
-- low-level datastore form that involves numerous indirection.
(
    blob_id,
    provenance
)
as
select
    b.blob_id,
    bp.provenance
from
    blob b
    inner join blob_provenance bp on (b.id = bp.blob_id)
;

CREATE VIEW finding_denorm
-- A convenience view for findings in their fully denormalized form rather
-- than the low-level datastore form that involves numerous indirection.
(
    finding_id,
    rule_name,
    rule_text_id,
    rule_structural_id,
    rule_syntax,
    groups,
    num_matches,
    num_redundant_matches,
    mean_score,
    comment,
    match_statuses
)
as
select
    f.finding_id,
    r.name,
    r.text_id,
    r.structural_id,
    r.syntax,
    f.groups,
    count(*),
    sum(case when m.id in (select match_id from match_redundancy) then 1 else 0 end),
    avg(ms.score),
    fc.comment,
    json_group_array(distinct match_status.status)
        filter (where match_status.status is not null) match_statuses
from
    finding f
    left outer join match m on (m.finding_id = f.id)
    left outer join rule r on (f.rule_id = r.id)
    left outer join match_score ms on (m.id = ms.match_id)
    left outer join match_status on (m.id = match_status.match_id)
    left outer join finding_comment fc on (f.id = fc.finding_id)
group by f.id
;


CREATE VIEW finding_summary
-- A convenience view for a summary of findings in denormalized form.
(
    rule_name,
    rule_structural_id,
    total_findings,
    total_matches,
    accept_findings,
    reject_findings,
    mixed_findings,
    unlabeled_findings
)
as
with
    -- table of relevant per-match information
    m as (
        select
            f.finding_id finding_id,
            r.name rule_name,
            r.structural_id rule_structural_id,
            ms.status match_status
        from
            finding f
            inner join match m on (m.finding_id = f.id)
            inner join rule r on (f.rule_id = r.id)
            left outer join match_status ms on (m.id = ms.match_id)
    ),
    -- summarize per-match information by finding
    f as (
        select
            finding_id,
            rule_name,
            rule_structural_id,
            case group_concat(distinct match_status)
                when 'accept' then 'accept'
                when 'reject' then 'reject'
                when 'accept,reject' then 'mixed'
                when 'reject,accept' then 'mixed'
            end finding_status,
            count(*) num_matches
        from m
        group by finding_id
    )
select
    rule_name,
    rule_structural_id,
    count(distinct finding_id) total_findings,
    sum(num_matches) total_matches,
    sum(case when finding_status = 'accept' then 1 else 0 end) accept_findings,
    sum(case when finding_status = 'reject' then 1 else 0 end) reject_findings,
    sum(case when finding_status = 'mixed' then 1 else 0 end) mixed_findings,
    sum(case when finding_status is null then 1 else 0 end) unlabeled_findings
from
    f
group by rule_name
