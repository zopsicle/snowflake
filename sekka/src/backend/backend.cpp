// _FORTIFY_SOURCE causes a warning with -O0.
#undef _FORTIFY_SOURCE

// SpiderMonkey emits a lot of warnings.
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Winvalid-offsetof"
#pragma GCC diagnostic ignored "-Wunused-parameter"
#include <jsapi.h>
#include <js/CompilationAndEvaluation.h>
#include <js/Initialization.h>
#include <js/SourceText.h>
#pragma GCC diagnostic pop

#include <cstddef>
#include <iostream>
#include <memory>
#include <stdexcept>

namespace
{
    /// Run JS_Init and JS_ShutDown at appropriate times.
    struct InitShutdown
    {
        InitShutdown()
        {
            JS_Init();
        }

        ~InitShutdown()
        {
            JS_ShutDown();
        }
    } InitShutdown;
}

class SekkaBackend
{
public:
    SekkaBackend(
        char const* runtime_js_ptr,
        std::size_t runtime_js_len
    )
        : context(JS_NewContext(JS::DefaultHeapMaxBytes))
    {
        if (context == nullptr)
            throw std::runtime_error("JS_NewContext: NULL");

        auto ok = JS::InitSelfHostedCode(context.get());
        if (!ok)
            throw std::runtime_error("JS::InitSelfHostedCode: false");

        JS::RealmOptions realm_options;

        JSClass global_class = {
            /* name  */ "Global",
            /* flags */ JSCLASS_GLOBAL_FLAGS,
            /* cOps  */ &JS::DefaultGlobalClassOps,
        };

        auto global_raw = JS_NewGlobalObject(
            /* cx         */ context.get(),
            /* clasp      */ &global_class,
            /* principals */ nullptr,
            /* hookOption */ JS::FireOnNewGlobalHook,
            /* options    */ realm_options
        );
        if (global_raw == nullptr)
            throw std::runtime_error("JS_NewGlobalObject: nullptr");

        JS::RootedObject global_rooted(context.get(), global_raw);

        // Scope guard that sets the current global object.
        JSAutoRealm realm(context.get(), global_rooted);

        JS::CompileOptions compile_options(context.get());
        compile_options.setFileAndLine("runtime.js", 1);

        JS::SourceText<mozilla::Utf8Unit> source;
        ok = source.init(
            /* cx          */ context.get(),
            /* units       */ runtime_js_ptr,
            /* unitsLength */ runtime_js_len,
            /* ownership   */ JS::SourceOwnership::Borrowed
        );
        if (!ok)
            throw std::runtime_error("JS::SourceText::init: false");

        JS::RootedValue result(context.get());

        ok = JS::Evaluate(
            /* cx      */ context.get(),
            /* options */ compile_options,
            /* srcBuf  */ source,
            /* rval    */ &result
        );
        if (!ok)
            throw std::runtime_error("JS::Evaluate: false");

        JS::RootedString result_string(context.get());

        result_string = JS_ValueToSource(context.get(), result);

        JS::UniqueChars bytes(JS_EncodeStringToUTF8(context.get(), result_string));

        std::cout << bytes.get() << "\n";

        throw 0;
    }

    ~SekkaBackend()
    {
    }

private:
    struct JSContextDelete
    {
        void operator()(JSContext* self) const
        {
            if (self != nullptr)
                JS_DestroyContext(self);
        }
    };
    std::unique_ptr<JSContext, JSContextDelete> context;
};

extern "C" SekkaBackend* sekka_backend_new(
    char const* runtime_js_ptr,
    std::size_t runtime_js_len
) noexcept
{
    return new SekkaBackend(runtime_js_ptr, runtime_js_len);
}

extern "C" void sekka_backend_drop(SekkaBackend* backend) noexcept
{
    delete backend;
}
