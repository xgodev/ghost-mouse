//! One element of another program's interface, read through Accessibility.

use std::{ffi::c_void, ptr::NonNull};

use objc2_application_services::{AXCopyMultipleAttributeOptions, AXUIElement, AXValue, AXValueType};
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

    /// Several attributes in one round trip to the program, in the order asked; `None` for each one the
    /// element does not have.
    pub(crate) fn attributes(&self, names: &[&str]) -> Vec<Option<CFRetained<CFType>>> {
        let list: Vec<_> = names.iter().map(|name| CFString::from_str(name)).collect();
        let list = CFArray::from_retained_objects(&list);
        let mut values: *const CFArray = std::ptr::null();
        // SAFETY: the out pointer is valid; on success it holds an array this call owns.
        let values = unsafe {
            let options = AXCopyMultipleAttributeOptions(0);
            let error = self.0.copy_multiple_attribute_values(list.as_opaque(), options, NonNull::from(&mut values));
            match NonNull::new(values.cast_mut()) {
                Some(values) if error.0 == 0 => CFRetained::from_raw(values),
                _ => return vec![None; names.len()],
            }
        };
        // SAFETY: an array of Core Foundation objects is what Accessibility hands out.
        let values = unsafe { values.cast_unchecked::<CFType>() };
        (0..names.len()).map(|index| values.get(index).filter(|value| !is_error(value))).collect()
    }

    pub(crate) fn elements(&self, name: &str) -> Vec<Element> {
        self.attribute(name).map(elements).unwrap_or_default()
    }

    pub(crate) fn text(&self, name: &str) -> Option<String> {
        Some(self.attribute(name)?.downcast::<CFString>().ok()?.to_string())
    }

    /// Turns a flag on and says whether the program took it.
    pub(crate) fn switch_on(&self, name: &str) -> bool {
        // SAFETY: both values are live Core Foundation objects.
        unsafe { self.0.set_attribute_value(&CFString::from_str(name), CFBoolean::new(true)) }.0 == 0
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
        frame(self.attribute("AXPosition")?, self.attribute("AXSize")?)
    }

    /// `true` when the element answered without an error. A file panel may answer with one for a press
    /// that worked, as it closes or changes folder: there, the outcome is what shows whether it landed.
    pub(crate) fn press(&self) -> bool {
        // SAFETY: the action name is a live string.
        unsafe { self.0.perform_action(&CFString::from_str("AXPress")) }.0 == 0
    }
}

/// The elements of an attribute value that lists them.
pub(crate) fn elements(value: CFRetained<CFType>) -> Vec<Element> {
    let Ok(array) = value.downcast::<CFArray>() else {
        return vec![];
    };
    // SAFETY: an array of Core Foundation objects is what Accessibility hands out.
    let array = unsafe { array.cast_unchecked::<CFArray<CFType>>() };
    array.iter().filter_map(|item| item.downcast::<AXUIElement>().ok()).map(Element).collect()
}

/// A rectangle from a position and a size value, in desktop points.
pub(crate) fn frame(position: CFRetained<CFType>, size: CFRetained<CFType>) -> Option<CGRect> {
    let mut origin = CGPoint::new(0.0, 0.0);
    let mut extent = CGSize::new(0.0, 0.0);
    let position = position.downcast::<AXValue>().ok()?;
    let size = size.downcast::<AXValue>().ok()?;
    // SAFETY: each out pointer is the kind the type asks for; a value of another type is refused.
    let read = unsafe {
        position.value(AXValueType::CGPoint, NonNull::from(&mut origin).cast::<c_void>())
            && size.value(AXValueType::CGSize, NonNull::from(&mut extent).cast::<c_void>())
    };
    read.then_some(CGRect::new(origin, extent))
}

/// An attribute read among several comes back as an error value when the element lacks it.
fn is_error(value: &CFType) -> bool {
    // SAFETY: reading the kind of a live value.
    value.downcast_ref::<AXValue>().is_some_and(|value| unsafe { value.r#type() } == AXValueType::AXError)
}
