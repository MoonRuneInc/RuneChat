-- Harden invite counters before invite logic lands in Plan 4.
-- max_uses = 0 is nonsensical (invite already exhausted at creation).
-- uses < 0 would be a decrement bug.
-- uses > max_uses means the enforcement gate failed — treat it as a DB invariant.
ALTER TABLE invites
    ADD CONSTRAINT invites_max_uses_positive
        CHECK (max_uses IS NULL OR max_uses > 0),
    ADD CONSTRAINT invites_uses_nonnegative
        CHECK (uses >= 0),
    ADD CONSTRAINT invites_uses_within_max
        CHECK (max_uses IS NULL OR uses <= max_uses);
