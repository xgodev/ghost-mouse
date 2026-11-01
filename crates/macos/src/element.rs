//! One element of another program's interface, read through Accessibility.

use std::{ffi::c_void, ptr::NonNull};

use objc2_application_services::{AXUIElement, AXValue, AXValueType};
use objc2_core_foundation::{CFArray, CFBoolean, CFRetained, CFString, CFType, CGPoint, CGRect, CGSize};

pub(crate) struct Element(pub(crate) CFRetained<AXUIElement>);

impl Element {
    pub(crate) fn attribute(&self, name: &str) -> Option<CFRetained<CFType>> {
        let mut value: *const CFType = std::ptr::null();
        // SAFETY: the out pointer is valid; on success it holds a value this call owns.
        unsafe {
            let error = self.0.copy_attribute_value(&CFString::from_str(name), NonNull::from(&mut value));
            if error.0 != 0 {
                return None;
            }
            NonNull::new(value.cast_mut()).map(|value| CFRetained::from_raw(value))
        }
    }

    pub(crate) fn elements(&self, name: &str) -> Vec<Element> {
        let Some(array) = self.attribute(name).and_then(|value| value.downcast::<CFArray>().ok()) else {
            return vec![];
        };
        // SAFETY: an array of Core Foundation objects is what Accessibility hands out.
        let array = unsafe { array.cast_unchecked::<CFArray<CFType>>() };
        array.iter().filter_map(|item| item.downcast::<AXUIElement>().ok()).map(Element).collect()
    }

    pub(crate) fn text(&self, name: &str) -> Option<String> {
        Some(self.attribute(name)?.downcast::<CFString>().ok()?.to_string())
    }

    pub(crate) fn flag(&self, name: &str) -> bool {
        let value = self.attribute(name).and_then(|value| value.downcast::<CFBoolean>().ok());
        value.is_some_and(|value| value.as_bool())
    }

    pub(crate) fn is(&self, role: &str) -> bool {
        self.text("AXRole").as_deref() == Some(role)
    }

    /// Where the element sits on the desktop, in points.
    pub(crate) fn frame(&self) -> Option<CGRect> {
        let mut origin = CGPoint::new(0.0, 0.0);
        let mut size = CGSize::new(0.0, 0.0);
        let position = self.attribute("AXPosition")?.downcast::<AXValue>().ok()?;
        let extent = self.attribute("AXSize")?.downcast::<AXValue>().ok()?;
        // SAFETY: each out pointer is the kind the type asks for; a value of another type is refused.
        let read = unsafe {
            position.value(AXValueType::CGPoint, NonNull::from(&mut origin).cast::<c_void>())
                && extent.value(AXValueType::CGSize, NonNull::from(&mut size).cast::<c_void>())
        };
        read.then_some(CGRect::new(origin, size))
    }

    /// `true` when the element answered without an error. A file panel may answer with one for a press
    /// that worked, as it closes or changes folder: there, the outcome is what shows whether it landed.
    pub(crate) fn press(&self) -> bool {
        // SAFETY: the action name is a live string.
        unsafe { self.0.perform_action(&CFString::from_str("AXPress")) }.0 == 0
    }
}
