(global => {

    function is_empty_string(value)
    {
        return (
            value instanceof Uint8Array &&
            value.length === 0
        );
    }

    function is_zero_string(value)
    {
        return (
            value instanceof Uint8Array &&
            value.length === 1          &&
            value[0] === 0x30  // b'0'
        );
    }

    global.sekka_to_boolean = value => {
        return (
            value !== null          &&  // undef
            value !== false         &&  // false
            value !== 0n            &&  // 0
            value !== 0.0           &&  // 0.0
            !is_empty_string(value) &&  // ''
            !is_zero_string(value)      // '0'
            // TODO: Empty array.
            // TODO: Empty hash.
        );
    };

})(this);
