-- This returns a sandbox environment to use with Modcrab.
-- This sandbox includes most of the Lua standard library.
-- The excluded things are mostly just coroutines, IO, and some OS stuff (such as 'os.execute').

return {
    -- Globals
    modcrab = modcrab,
    _G = _G,
    _VERSION = _VERSION,

    -- Basic Functions
    assert = assert,
    error = error,
    getmetatable = getmetatable,
    ipairs = ipairs,
    next = next,
    pairs = pairs,
    pcall = pcall,
    print = print,
    rawequal = rawequal,
    rawget = rawget,
    rawlen = rawlen,
    rawset = rawset,
    select = select,
    setmetatable = setmetatable,
    tonumber = tonumber,
    tostring = tostring,
    type = type,
    xpcall = xpcall,

    -- Coroutines are not included sandbox, but they could be placed here if needed.

    -- Strings
    string = {
        byte = string.byte,
        char = string.char,
        dump = string.dump,
        find = string.find,
        format = string.format,
        gmatch = string.gmatch,
        gsub = string.gsub,
        len = string.len,
        lower = string.lower,
        match = string.match,
        pack = string.pack,
        packsize = string.packsize,
        rep = string.rep,
        reverse = string.reverse,
        sub = string.sub,
        unpack = string.unpack,
        upper = string.upper,
    },

    -- Tables
    table = {
        concat = table.concat,
        insert = table.insert,
        move = table.move,
        pack = table.pack,
        remove = table.remove,
        sort = table.sort,
        unpack = table.unpack,
    },

    -- Math
    math = {
        abs = math.abs,
        acos = math.acos,
        asin = math.asin,
        atan = math.atan,
        ceil = math.ceil,
        cos = math.cos,
        deg = math.deg,
        exp = math.exp,
        floor = math.floor,
        fmod = math.fmod,
        huge = math.huge,
        log = math.log,
        max = math.max,
        maxinteger = math.maxinteger,
        min = math.min,
        mininteger = math.mininteger,
        modf = math.modf,
        pi = math.pi,
        rad = math.rad,
        random = math.random,
        randomseed = math.randomseed,
        sin = math.sin,
        sqrt = math.sqrt,
        tan = math.tan,
        tointeger = math.tointeger,
        type = math.type,
        ult = math.ult,
    },

    -- The IO library is completely excluded.

    -- OS
    os = {
        clock = os.clock,
        data = os.date,
        difftime = os.difftime,
        getenv = os.getenv,
    },
}
