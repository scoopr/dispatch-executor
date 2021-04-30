
SYSROOT=$(xcrun --show-sdk-path)

FOUNDATION_HEADER="$SYSROOT/System/Library/Frameworks/Foundation.framework/Headers/Foundation.h"

BINDGEN_FLAGS="--allowlist-type (.*?NS(RunLoop|Object|Value|Proxy|OrderedSet|Date|Formatter).*) --allowlist-var .*?NS(.*?RunLoop.*?Mode.*) -o foundation.rs"

CLANG_ARGS="-isysroot $SYSROOT -x objective-c"
echo bindgen $BINDGEN_FLAGS $FOUNDATION_HEADER -- $CLANG_ARGS
bindgen $BINDGEN_FLAGS $FOUNDATION_HEADER --raw-line "use objc::{msg_send,sel,sel_impl,class};" -- $CLANG_ARGS