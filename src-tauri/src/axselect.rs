//! Locating the on-screen rectangle of the user's current text selection via
//! the macOS Accessibility API, so the overlay can be anchored to the text
//! rather than to the mouse cursor.
//!
//! This is strictly best-effort: many apps (notably browsers rendering web
//! content) don't expose `AXBoundsForRange`, in which case we return `None` and
//! the caller falls back to the cursor position.

/// The screen rectangle (logical points, top-left origin) of the currently
/// selected text in the frontmost app, or `None` if it can't be determined.
///
/// Coordinates match `CGDisplay` bounds and the Quartz cursor position, so the
/// result is directly comparable with the rest of the placement code.
#[cfg(target_os = "macos")]
pub fn selected_text_bounds() -> Option<(f64, f64, f64, f64)> {
    use accessibility_sys::{
        kAXBoundsForRangeParameterizedAttribute, kAXErrorSuccess, kAXFocusedUIElementAttribute,
        kAXSelectedTextRangeAttribute, kAXValueTypeCGRect,
        AXUIElementCopyParameterizedAttributeValue, AXUIElementCreateSystemWide, AXUIElementRef,
        AXValueGetValue, AXValueRef,
    };
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    use core_foundation_sys::base::{CFRelease, CFTypeRef};
    use core_graphics::geometry::CGRect;

    unsafe {
        let system = AXUIElementCreateSystemWide();
        if system.is_null() {
            return None;
        }

        // system-wide element → the focused UI element.
        let focused = match copy_attr(system, kAXFocusedUIElementAttribute) {
            Some(p) => p as AXUIElementRef,
            None => {
                CFRelease(system as CFTypeRef);
                return None;
            }
        };
        CFRelease(system as CFTypeRef);

        // focused element → its selected text range (an AXValue wrapping CFRange).
        let range = match copy_attr(focused, kAXSelectedTextRangeAttribute) {
            Some(p) => p,
            None => {
                CFRelease(focused as CFTypeRef);
                return None;
            }
        };

        // (focused element, range) → the bounding rect of that range.
        let attr = CFString::new(kAXBoundsForRangeParameterizedAttribute);
        let mut bounds_ref: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyParameterizedAttributeValue(
            focused,
            attr.as_concrete_TypeRef(),
            range,
            &mut bounds_ref,
        );
        CFRelease(range);
        CFRelease(focused as CFTypeRef);

        if err != kAXErrorSuccess || bounds_ref.is_null() {
            return None;
        }

        let mut rect = CGRect::default();
        let ok = AXValueGetValue(
            bounds_ref as AXValueRef,
            kAXValueTypeCGRect,
            &mut rect as *mut CGRect as *mut std::ffi::c_void,
        );
        CFRelease(bounds_ref);

        if !ok || rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return None;
        }

        Some((
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        ))
    }
}

/// Copy an attribute off an AXUIElement, returning its raw `CFTypeRef` (owned by
/// the caller, must be `CFRelease`d) or `None` on any error / null value.
#[cfg(target_os = "macos")]
unsafe fn copy_attr(
    element: accessibility_sys::AXUIElementRef,
    attribute: &str,
) -> Option<core_foundation_sys::base::CFTypeRef> {
    use accessibility_sys::{kAXErrorSuccess, AXUIElementCopyAttributeValue};
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    use core_foundation_sys::base::CFTypeRef;

    let name = CFString::new(attribute);
    let mut value: CFTypeRef = std::ptr::null();
    let err = AXUIElementCopyAttributeValue(element, name.as_concrete_TypeRef(), &mut value);
    if err != kAXErrorSuccess || value.is_null() {
        None
    } else {
        Some(value)
    }
}

#[cfg(not(target_os = "macos"))]
pub fn selected_text_bounds() -> Option<(f64, f64, f64, f64)> {
    None
}
