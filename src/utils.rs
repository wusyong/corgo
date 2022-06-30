use std::sync::atomic::AtomicUsize;

bitflags::bitflags! {
    pub struct ClassID: usize {
        const OBJECT                   = 1;
        const ARRAY                    = 2;   /* u.array       | length */
        const ERROR                    = 3;
        const NUMBER                   = 4;   /* u.object_data */
        const STRING                   = 5;   /* u.object_data */
        const BOOLEAN                  = 6;   /* u.object_data */
        const SYMBOL                   = 7;   /* u.object_data */
        const ARGUMENTS                = 8;   /* u.array       | length */
        const MAPPED_ARGUMENTS         = 9;   /*               | length */
        const DATE                     = 10;  /* u.object_data */
        const MODULE_NS                = 11;
        const C_FUNCTION               = 12;  /* u.cfunc */
        const BYTECODE_FUNCTION        = 13;  /* u.func */
        const BOUND_FUNCTION           = 14;  /* u.bound_function */
        const C_FUNCTION_DATA          = 15;  /* u.c_function_data_record */
        const GENERATOR_FUNCTION       = 16;  /* u.func */
        const FOR_IN_ITERATOR          = 17;  /* u.for_in_iterator */
        const REGEXP                   = 18;  /* u.regexp */
        const ARRAY_BUFFER             = 19;  /* u.array_buffer */
        const SHARED_ARRAY_BUFFER      = 20;  /* u.array_buffer */
        const UINT8C_ARRAY             = 21;  /* u.array (typed_array) */
        const INT8_ARRAY               = 22;  /* u.array (typed_array) */
        const UINT8_ARRAY              = 23;  /* u.array (typed_array) */
        const INT16_ARRAY              = 24;  /* u.array (typed_array) */
        const UINT16_ARRAY             = 25;  /* u.array (typed_array) */
        const INT32_ARRAY              = 26;  /* u.array (typed_array) */
        const UINT32_ARRAY             = 27;  /* u.array (typed_array) */
        const BIG_INT64_ARRAY          = 28;  /* u.array (typed_array) */
        const BIG_UINT64_ARRAY         = 29;  /* u.array (typed_array) */
        const FLOAT32_ARRAY            = 30;  /* u.array (typed_array) */
        const FLOAT64_ARRAY            = 31;  /* u.array (typed_array) */
        const DATAVIEW                 = 32;  /* u.typed_array */
        const BIG_INT                  = 33;  /* u.object_data */
        const BIG_FLOAT                = 34;  /* u.object_data */
        const FLOAT_ENV                = 35;  /* u.float_env */
        const BIG_DECIMAL              = 36;  /* u.object_data */
        const OPERATOR_SET             = 37;  /* u.operator_set */
        const MAP                      = 38;  /* u.map_state */
        const SET                      = 39;  /* u.map_state */
        const WEAKMAP                  = 40;  /* u.map_state */
        const WEAKSET                  = 41;  /* u.map_state */
        const MAP_ITERATOR             = 42;  /* u.map_iterator_data */
        const SET_ITERATOR             = 43;  /* u.map_iterator_data */
        const ARRAY_ITERATOR           = 44;  /* u.array_iterator_data */
        const STRING_ITERATOR          = 45;  /* u.array_iterator_data */
        const REGEXP_STRING_ITERATOR   = 46;  /* u.regexp_string_iterator_data */
        const GENERATOR                = 47;  /* u.generator_data */
        const PROXY                    = 48;  /* u.proxy_data */
        const PROMISE                  = 49;  /* u.promise_data */
        const PROMISE_RESOLVE_FUNCTION = 50;  /* u.promise_function_data */
        const PROMISE_REJECT_FUNCTION  = 51;  /* u.promise_function_data */
        const ASYNC_FUNCTION           = 52;  /* u.func */
        const ASYNC_FUNCTION_RESOLVE   = 53;  /* u.async_function_data */
        const ASYNC_FUNCTION_REJECT    = 54;  /* u.async_function_data */
        const ASYNC_FROM_SYNC_ITERATOR = 55;  /* u.async_from_sync_iterator_data */
        const ASYNC_GENERATOR_FUNCTION = 56;  /* u.func */
        const ASYNC_GENERATOR          = 57;  /* u.async_generator_data */
        const INIT_COUNT               = 58; /* last entry for predefined classes */
    }
}

pub const NEW_CLASS_ID: AtomicUsize = AtomicUsize::new(ClassID::INIT_COUNT.bits());

pub enum Error {
    Eval,
    Range,
    Reference,
    Syntax,
    Type,
    URI,
    Internal,
    Aggregate,
    /// Number of different NativeError objects
    NativeErrorCount,
}
