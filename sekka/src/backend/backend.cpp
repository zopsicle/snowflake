// _FORTIFY_SOURCE causes a warning with -O0.
#undef _FORTIFY_SOURCE

// Nixpkgs enables this V8 option, so we must too.
#define V8_COMPRESS_POINTERS

// Include V8 headers.
#include <libplatform/libplatform.h>
#include <v8.h>

// Include standard library.
#include <cstddef>
#include <memory>

// Utility for defining deleters.
#define SEKKA_DEFINE_DELETER(name, body) \
    struct name                          \
    {                                    \
        template<typename T>             \
        void operator()(T* self) const   \
        {                                \
            body                         \
        }                                \
    }

/// Initialize any global variables.
///
/// This must be called before any of the other functions.
extern "C" void sekka_backend_init() noexcept
{
    // Initialize the ICU Unicode library bundled with V8.
    // Nixpkgs ships the ICU data alongside V8,
    // so we don't need to pass a path here.
    v8::V8::InitializeICU();

    // Nixpkgs disables V8 external startup data, so don't call this.
    // v8::V8::InitializeExternalStartupData();

    // Initialize V8's platform abstraction layer.
    // The platform must stay alive after we return.
    static auto platform = v8::platform::NewDefaultPlatform();
    v8::V8::InitializePlatform(platform.get());

    // Initialize remaining V8 globals.
    v8::V8::Initialize();
}

struct SekkaBackend
{
    SekkaBackend()
    {
        // Tells V8 how to allocate ArrayBuffer backing stores.
        // The default allocator uses malloc and free which is fine.
        array_buffer_allocator.reset(
            v8::ArrayBuffer::Allocator::NewDefaultAllocator()
        );

        // Create the V8 isolate for this backend.
        // An isolate is a JavaScript virtual machine with its own heap.
        v8::Isolate::CreateParams create_params;
        create_params.array_buffer_allocator = array_buffer_allocator.get();
        isolate.reset(v8::Isolate::New(create_params));

        // Set V8 isolate and handle scopes.
        v8::Isolate::Scope isolate_scope(isolate.get());
        v8::HandleScope handle_scope(isolate.get());

        // Create the V8 context for this backend.
        // A context maintains a JavaScript global object.
        context.Set(isolate.get(), v8::Context::New(isolate.get()));
    }

    /// Compile and run JavaScript code.
    bool run_js(char const* js_ptr, std::size_t js_len)
    {
        // Set V8 isolate, handle, and context scopes.
        v8::Isolate::Scope isolate_scope(isolate.get());
        v8::HandleScope handle_scope(isolate.get());
        auto context = this->context.Get(isolate.get());
        v8::Context::Scope context_scope(context);

        // Wrap the JavaScript code in a V8 string.
        auto maybe_js_source = v8::String::NewFromUtf8(
            /* isolate */ isolate.get(),
            /* data    */ js_ptr,
            /* type    */ v8::NewStringType::kNormal,
            /* length  */ js_len
        );
        v8::Local<v8::String> js_source;
        if (!maybe_js_source.ToLocal(&js_source))
            return false;

        // Compile the JavaScript code to a V8 script.
        auto maybe_js_script = v8::Script::Compile(context, js_source);
        v8::Local<v8::Script> js_script;
        if (!maybe_js_script.ToLocal(&js_script))
            return false;

        // Run the compiled JavaScript code.
        auto maybe_result = js_script->Run(context);
        v8::Local<v8::Value> result;
        if (!maybe_result.ToLocal(&result))
            return false;

        return true;
    }

    SEKKA_DEFINE_DELETER(IsolateDispose, { self->Dispose(); });

    std::unique_ptr<v8::ArrayBuffer::Allocator> array_buffer_allocator;
    std::unique_ptr<v8::Isolate, IsolateDispose> isolate;
    v8::Eternal<v8::Context> context;
};

/// Create a backend.
///
/// Returns null if backend creation failed.
extern "C" SekkaBackend* sekka_backend_new() noexcept
try {
    return new SekkaBackend();
} catch (...) {
    return nullptr;
}

/// Drop a backend.
extern "C" void sekka_backend_drop(SekkaBackend* backend) noexcept
{
    delete backend;
}

/// Run JavaScript code.
///
/// Returns false if running the code failed.
extern "C" bool sekka_backend_run_js(
    SekkaBackend* backend,
    char const* js_ptr,
    std::size_t js_len
) noexcept
try {
    return backend->run_js(js_ptr, js_len);
} catch (...) {
    return false;
}
