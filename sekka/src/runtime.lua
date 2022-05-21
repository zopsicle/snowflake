-- ========================
-- Representation of values
-- ========================
--
-- Every Sekka value is represented by a Lua value.
-- This chapter describes this correspondence.
--
-- Sekka undef is represented by the Lua ``sekka_undef`` table.
-- Sekka Booleans are represented by Lua Booleans.
-- Sekka integers are represented by TODO.
-- Sekka floats are represented by Lua floats.
-- Sekka subroutines are represented by Lua functions.
-- Sekka arrays are represented by TODO.
-- Sekka hashes are represented by TODO.
--
-- Because Lua nil has all sorts of special behavior,
-- no Sekka value is ever represented by Lua nil.

sekka_undef       = { }
sekka_array_empty = { }
sekka_hash_empty  = { }

function sekka_to_boolean(value)
    return (
        value ~= sekka_undef       and
        value ~= false             and
        -- TODO: Integer zero.
        value ~= 0.0               and
        value ~= ""                and
        value ~= "0"               and
        value ~= sekka_array_empty and
        value ~= sekka_hash_empty
    )
end

function sekka_to_numeric(value)
end

function sekka_to_string(value)
end
