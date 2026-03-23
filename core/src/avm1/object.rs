//! Object trait to expose objects to AVM

use crate::avm1::function::{ExecutionName, ExecutionReason, FunctionObject};
use crate::avm1::globals::bevel_filter::BevelFilter;
use crate::avm1::globals::blur_filter::BlurFilter;
use crate::avm1::globals::color_matrix_filter::ColorMatrixFilter;
use crate::avm1::globals::color_transform::ColorTransformObject;
use crate::avm1::globals::convolution_filter::ConvolutionFilter;
use crate::avm1::globals::date::Date;
use crate::avm1::globals::displacement_map_filter::DisplacementMapFilter;
use crate::avm1::globals::drop_shadow_filter::DropShadowFilter;
use crate::avm1::globals::file_reference::FileReferenceObject;
use crate::avm1::globals::glow_filter::GlowFilter;
use crate::avm1::globals::gradient_filter::GradientFilter;
use crate::avm1::globals::local_connection::LocalConnection;
use crate::avm1::globals::netconnection::NetConnection;
use crate::avm1::globals::shared_object::SharedObject;
use crate::avm1::globals::sound::Sound;
use crate::avm1::globals::style_sheet::StyleSheetObject;
use crate::avm1::globals::text_snapshot::TextSnapshotObject;
use crate::avm1::globals::transform::TransformObject;
use crate::avm1::globals::xml::Xml;
use crate::avm1::globals::xml_socket::XmlSocket;
use crate::avm1::object::super_object::SuperObject;
use crate::avm1::xml::XmlNode;
use crate::avm1::{Activation, Error, Value};
use crate::bitmap::bitmap_data::BitmapData;
use crate::display_object::{
    Avm1Button, DisplayObject, EditText, MovieClip, TDisplayObject as _, Video,
};
use crate::html::TextFormat;
use crate::streams::NetStream;
use crate::string::AvmString;
use gc_arena::{Collect, Gc, Mutation};
use ruffle_macros::istr;
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;

mod script_object;
pub mod stage_object;
pub mod super_object;

pub use script_object::{Object, ObjectHandle, ObjectWeak};

use js_sys::{Reflect, Array};
use crate::LingoCallback;
use crate::LINGO_CALLBACKS;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use base64::{Engine};
use js_sys::JSON;

use crate::external::Value as ExternalValue;

pub fn external_to_js_value(external: ExternalValue) -> JsValue {
    match external {
        ExternalValue::Undefined => JsValue::UNDEFINED,
        ExternalValue::Null => JsValue::NULL,
        ExternalValue::Bool(value) => JsValue::from_bool(value),
        ExternalValue::Number(value) => JsValue::from_f64(value),
        ExternalValue::String(value) => JsValue::from_str(&value),
        ExternalValue::Object(map) => {
            // Detect Denizen type
            let is_denizen = map.get("#type").map_or(false, |v| match v {
                ExternalValue::String(s) => s == "Denizen",
                _ => false,
            });

            // Create JS object
            let js_obj = if is_denizen {
                let ctor = js_sys::Reflect::get(
                    &js_sys::global(),
                    &JsValue::from_str("Denizen")
                )
                .ok()
                .and_then(|ctor| ctor.dyn_into::<js_sys::Function>().ok());

                if let Some(ctor) = ctor {
                    Reflect::construct(&ctor, &js_sys::Array::new()).unwrap_or(JsValue::NULL)
                } else {
                    js_sys::Object::new().into()
                }
            } else {
                js_sys::Object::new().into()
            };

            // Populate properties
            for (k, v) in map {
                if k == "#type" { continue; }
                let _ = js_sys::Reflect::set(&js_obj, &JsValue::from_str(&k), &external_to_js_value(v));
            }

            js_obj
        }
        ExternalValue::List(values) => {
            let arr = js_sys::Array::new();
            for v in values {
                arr.push(&external_to_js_value(v));
            }
            arr.into()
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "triggerLingoCallbackOnScript")]
    fn trigger_lingo_callback_on_script(
        cast_lib: i32,
        cast_member: i32,
        handler_name: String,
        args: String,
        flash_cast_lib: i32,
        flash_cast_member: i32,
    );
}

#[derive(Copy, Clone, Collect)]
#[collect(no_drop)]
pub enum NativeObject<'gc> {
    None,

    /// A `super` object, used to call superclass methods.
    ///
    /// `super` objects should never have any properties (including `__proto__`); instead,
    /// relevant operations are forwarded to the `SuperObject`'s target.
    Super(SuperObject<'gc>),
    /// A boxed boolean.
    Bool(bool),
    /// A boxed number.
    Number(BoxedF64<'gc>),
    /// A boxed string.
    String(AvmString<'gc>),
    /// Marker indicating that this object should behave like an array.
    ///
    /// This stores no data; all array properties are stored on the object's main `PropertyMap`.
    /// TODO(moulins): that doesn't seem entirely correct; in Flash Player, it is possible in
    /// certain circumstances (e.g. in a subclass constructor, before calling `super()`) to
    /// 'desynchronize' the "property view" and the "array view" (used by, e.g., `toString()`).
    Array(()),
    Function(Gc<'gc, FunctionObject<'gc>>),

    MovieClip(MovieClip<'gc>),
    Button(Avm1Button<'gc>),
    EditText(EditText<'gc>),
    Video(Video<'gc>),

    Date(Gc<'gc, Cell<Date>>),
    BlurFilter(BlurFilter<'gc>),
    BevelFilter(BevelFilter<'gc>),
    GlowFilter(GlowFilter<'gc>),
    DropShadowFilter(DropShadowFilter<'gc>),
    ColorMatrixFilter(ColorMatrixFilter<'gc>),
    DisplacementMapFilter(DisplacementMapFilter<'gc>),
    ConvolutionFilter(ConvolutionFilter<'gc>),
    GradientBevelFilter(GradientFilter<'gc>),
    GradientGlowFilter(GradientFilter<'gc>),
    ColorTransform(Gc<'gc, ColorTransformObject>),
    Transform(TransformObject<'gc>),
    TextFormat(Gc<'gc, RefCell<TextFormat>>),
    NetStream(NetStream<'gc>),
    BitmapData(BitmapData<'gc>),
    Xml(Xml<'gc>),
    XmlNode(XmlNode<'gc>),
    SharedObject(Gc<'gc, RefCell<SharedObject>>),
    XmlSocket(XmlSocket<'gc>),
    FileReference(FileReferenceObject<'gc>),
    NetConnection(NetConnection<'gc>),
    LocalConnection(LocalConnection<'gc>),
    Sound(Sound<'gc>),
    StyleSheet(StyleSheetObject<'gc>),
    TextSnapshot(TextSnapshotObject<'gc>),
}

const _: () = assert!(size_of::<NativeObject<'_>>() <= size_of::<[usize; 2]>());

/// Small wrapper struct to keep boxed f64s word-sized on every architecture.
#[derive(Copy, Clone, Collect)]
#[collect(no_drop)]
pub struct BoxedF64<'gc> {
    #[cfg(target_pointer_width = "64")]
    value: f64,
    #[cfg(not(target_pointer_width = "64"))]
    value: Gc<'gc, f64>,
    _marker: PhantomData<Gc<'gc, ()>>,
}

impl<'gc> BoxedF64<'gc> {
    #[inline]
    pub fn new(#[allow(unused)] mc: &Mutation<'gc>, value: f64) -> Self {
        Self {
            #[cfg(target_pointer_width = "64")]
            value,
            #[cfg(not(target_pointer_width = "64"))]
            value: Gc::new(mc, value),
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn value(self) -> f64 {
        #[cfg(target_pointer_width = "64")]
        return self.value;
        #[cfg(not(target_pointer_width = "64"))]
        return *self.value;
    }
}

impl<'gc> NativeObject<'gc> {
    pub fn as_display_object(self) -> Option<DisplayObject<'gc>> {
        match self {
            Self::MovieClip(dobj) => Some(DisplayObject::MovieClip(dobj)),
            Self::Button(dobj) => Some(DisplayObject::Avm1Button(dobj)),
            Self::EditText(dobj) => Some(DisplayObject::EditText(dobj)),
            Self::Video(dobj) => Some(DisplayObject::Video(dobj)),
            _ => None,
        }
    }
}

impl<'gc> Object<'gc> {
    fn matches_callback(&self, name_str: &str, bean_manager: &str, callback: &LingoCallback) -> bool {
        if name_str == "beanCreated" {
            callback.method_name.eq_ignore_ascii_case(name_str) &&
            callback.movie_clip_path.contains(bean_manager)
        } else {
            callback.method_name.eq_ignore_ascii_case(name_str)
        }
    }
}

impl<'gc> Object<'gc> {
    /// Retrieve a named property from the object, or its prototype.
    /// Returns `None` if the property couldn't be found.
    pub fn get_opt(
        self,
        name: impl Into<AvmString<'gc>>,
        activation: &mut Activation<'_, 'gc>,
        call_resolve_fn: bool,
    ) -> Result<Option<Value<'gc>>, Error<'gc>> {
        // This duplicates logic already present in `SuperObject::proto` and so doesn't seem necessary.
        // But removing it would make `SuperObject`s go through an extra iteration in `search_prototype`,
        // which impacts the maximum possible depth before `Error::PrototypeRecursionLimit`.
        // TODO(moulins): Test this limit and figure out what is correct.
        let (this, proto) = if let Some(super_object) = self.as_super_object() {
            (super_object.this(), super_object.proto(activation))
        } else {
            (self, Value::Object(self))
        };

        let result = search_prototype(proto, name.into(), activation, this, call_resolve_fn)?;
        Ok(result.map(|(value, _depth)| value))
    }

    /// Retrieve a named property from the object, or its prototype.
    /// If the property couldn't be found, try to find and call a `__resolve` handler.
    pub fn get(
        self,
        name: impl Into<AvmString<'gc>>,
        activation: &mut Activation<'_, 'gc>,
    ) -> Result<Value<'gc>, Error<'gc>> {
        self.get_opt(name, activation, true)
            .map(|v| v.unwrap_or(Value::Undefined))
    }

    /// Retrieve a non-virtual property from the object, or its prototype.
    pub fn get_stored(
        self,
        name: AvmString<'gc>,
        activation: &mut Activation<'_, 'gc>,
    ) -> Result<Value<'gc>, Error<'gc>> {
        let mut depth = 0;
        let mut proto = Value::Object(self);

        while let Value::Object(p) = proto {
            if depth == 255 {
                return Err(Error::PrototypeRecursionLimit);
            }

            if let Some(value) = p.get_local_stored(name, activation) {
                return Ok(value);
            }

            proto = p.proto(activation);
            depth += 1;
        }

        Ok(Value::Undefined)
    }

    /// Set a named property on this object, or its prototype.
    pub fn set(
        self,
        name: impl Into<AvmString<'gc>>,
        value: Value<'gc>,
        activation: &mut Activation<'_, 'gc>,
    ) -> Result<(), Error<'gc>> {
        let name = name.into();
        if name.is_empty() {
            return Ok(());
        }

        let mut value = value;
        let (this, mut proto) = if let Some(super_object) = self.as_super_object() {
            (super_object.this(), super_object.proto(activation))
        } else {
            (self, Value::Object(self))
        };
        let watcher_result = self.call_watcher(activation, name, &mut value, this);

        if !self.has_own_property(activation, name) {
            // Before actually inserting a new property, we need to crawl the
            // prototype chain for virtual setters.
            while let Value::Object(this_proto) = proto {
                if this_proto.has_own_virtual(activation, name) {
                    if let Some(setter) = this_proto.setter(name, activation)
                        && let Some(exec) = setter.as_function()
                    {
                        exec.exec(
                            ExecutionName::Static("[Setter]"),
                            activation,
                            this.into(),
                            1,
                            &[value],
                            ExecutionReason::Special,
                            setter,
                        )?;
                    }
                    return Ok(());
                }

                proto = this_proto.proto(activation);
            }
        }

        let result = self.set_local(name, value, activation, this);
        watcher_result.and(result)
    }

    /// Call a method on the object.
    ///
    /// It is highly recommended to use this convenience method to perform
    /// method calls. It is morally equivalent to an AVM1 `ActionCallMethod`
    /// opcode. It will take care of retrieving the method, calculating its
    /// base prototype for `super` calls, and providing it with the correct
    /// `this` parameter.
    pub fn call_method(
        self,
        name: AvmString<'gc>,
        args: &[Value<'gc>],
        activation: &mut Activation<'_, 'gc>,
        reason: ExecutionReason,
    ) -> Result<Value<'gc>, Error<'gc>> {
        match self.native_no_super() {
            NativeObject::Super(zuper) => return zuper.call_method(name, args, activation, reason),
            native => {
                if native
                    .as_display_object()
                    .is_some_and(|dobj| dobj.avm1_removed())
                {
                    return Ok(Value::Undefined);
                }
            }
        }

        let name_clone = name.clone();
        let name_str = name_clone.to_utf8_lossy();
        let mut bean_manager: String = "".to_string();

        // Enhanced logging section for beanCreated calls
        if name_str == "beanCreated" {
            tracing::trace!("beanCreated() called on object: {:p}", self.as_ptr());

            // Look for the key identifier - sClass property
            match self.get_stored(AvmString::new_utf8(activation.context.gc_context, "sClass"), activation) {
                Ok(value) => {
                    let class_name = value.coerce_to_string(activation)
                        .map(|s| s.to_utf8_lossy().to_string())
                        .unwrap_or_else(|_| "unknown".to_string());
                    if let Some(manager_name) = class_name.split('.').last() {
                        let manager_name = manager_name.replace("Possession", "Posession");
                        let formatted_name = manager_name.replace("Flash", "o");
                        tracing::trace!("   MANAGER TYPE IDENTIFIED: {}", formatted_name);
                        bean_manager = formatted_name;
                    } else {
                        tracing::trace!("   MANAGER TYPE IDENTIFIED: {}", class_name);
                    }
                },
                Err(_) => tracing::trace!("   No sClass found - cannot identify manager type"),
            }
        }

        let args_clone: Vec<Value<'gc>> = args.iter().cloned().collect();
        // Lingo callback handling
        {
            let callbacks_snapshot: Vec<_> = {
                match LINGO_CALLBACKS.lock() {
                    Ok(guard) => guard.clone(),
                    Err(poisoned) => poisoned.into_inner().clone(),
                }
            };

            for callback in callbacks_snapshot.iter() {
                if !self.matches_callback(&name_str, &bean_manager, callback) {
                    continue;
                }

                tracing::trace!(
                    "Found matching Lingo callback! Triggering mcp: {} methodName: {} castLib: {} castMember: {} lingoHandler: {}",
                    callback.movie_clip_path,
                    callback.method_name,
                    callback.lingo_cast_lib,
                    callback.lingo_cast_member,
                    callback.lingo_handler
                );

                let js_args_array = {
                    let array = Array::new();
                    for arg in args_clone.iter() {
                        let ext_val = activation.store_and_convert_for_lingo(arg.to_owned());
                        let js_val = external_to_js_value(ext_val);
                        let json_str = JSON::stringify(&js_val)
                            .ok()
                            .and_then(|s| s.as_string())
                            .unwrap_or_else(|| "null".to_string());
                        let b64 = base64::engine::general_purpose::STANDARD.encode(json_str);
                        array.push(&JsValue::from_str(&b64));
                    }
                    array
                };

                let js_args_json = JSON::stringify(&js_args_array)
                    .ok()
                    .and_then(|s| s.as_string())
                    .unwrap_or_else(|| "[]".to_string());
                tracing::trace!("JSON string being added to queue (object): {}", js_args_json);

                trigger_lingo_callback_on_script(
                    callback.lingo_cast_lib,
                    callback.lingo_cast_member,
                    callback.lingo_handler.clone(),
                    js_args_json,
                    callback.flash_cast_lib,
                    callback.flash_cast_member,
                );
            }
        }

        // 'special' method calls appear to skip the `__resolve` fallback logic
        let call_resolve_fn = !matches!(reason, ExecutionReason::Special);
        let (method, depth) = match search_prototype(Value::Object(self), name, activation, self, call_resolve_fn)? {
            Some((Value::Object(method), depth)) => (method, depth),
            _ => {
                return Ok(Value::Undefined);
            }
        };

        // If the method was found on the object itself, change `depth` as-if
        // the method was found on the object's prototype.
        let depth = depth.max(1);

        match method.as_function() {
            Some(exec) => exec.exec(
                ExecutionName::Dynamic(name),
                activation,
                self.into(),
                depth,
                args,
                reason,
                method,
            ),
            None => {
                tracing::warn!(
                    "Property '{}' on object {:p} exists but is not executable.",
                    name_str,
                    self.as_ptr()
                );
                return Ok(Value::Undefined);
            }
        }
    }

    /// Determine if this object is an instance of a class.
    ///
    /// The class is provided in the form of its constructor function and the
    /// explicit prototype of that constructor function. It is assumed that
    /// they are already linked.
    ///
    /// Because ActionScript 2.0 added interfaces, this function cannot simply
    /// check the prototype chain and call it a day: each step in the chain has
    /// its own attached 'interface tree' which also needs to be checked.
    pub fn is_instance_of(
        self,
        activation: &mut Activation<'_, 'gc>,
        prototype: Object<'gc>,
    ) -> Result<bool, Error<'gc>> {
        // TODO(moulins): should we guard against infinite loops here?
        // A recursive prototype and/or interface chain will hang Flash Player.

        let mut interface_stack = smallvec::SmallVec::<[_; 4]>::new();
        let mut this = self;

        while let Value::Object(this_proto) = this.proto(activation) {
            interface_stack.push(this_proto);

            while let Some(interface) = interface_stack.pop() {
                if Object::ptr_eq(interface, prototype) {
                    return Ok(true);
                }

                interface_stack.extend(interface.interfaces().iter().cloned());
            }

            this = this_proto;
        }

        Ok(false)
    }

    /// Get the underlying XML node for this object, if it exists.
    pub fn as_xml_node(self) -> Option<XmlNode<'gc>> {
        match self.native() {
            NativeObject::Xml(xml) => Some(xml.root()),
            NativeObject::XmlNode(xml_node) => Some(xml_node),
            _ => None,
        }
    }

    /// Check if this object is in the prototype chain of the specified test object.
    pub fn is_prototype_of(self, activation: &mut Activation<'_, 'gc>, other: Object<'gc>) -> bool {
        // TODO(moulins): should we guard against infinite loops here?
        // A recursive prototype chain will hang Flash Player.

        let mut proto = other.proto(activation);

        while let Value::Object(proto_ob) = proto {
            if std::ptr::eq(self.as_ptr(), proto_ob.as_ptr()) {
                return true;
            }

            proto = proto_ob.proto(activation);
        }

        false
    }

    pub fn ptr_eq(a: Object<'gc>, b: Object<'gc>) -> bool {
        std::ptr::eq(a.as_ptr(), b.as_ptr())
    }
}

pub enum ObjectPtr {}

/// Perform a prototype lookup of a given object.
///
/// This function returns both the `Value` and the prototype depth from which
/// it was grabbed from. If the property did not resolve, then it returns
/// `Ok(None)`.
///
/// The prototype depth can and should be used to populate the `depth`
/// parameter necessary to make `super` work.
pub fn search_prototype<'gc>(
    mut proto: Value<'gc>,
    name: AvmString<'gc>,
    activation: &mut Activation<'_, 'gc>,
    this: Object<'gc>,
    call_resolve_fn: bool,
) -> Result<Option<(Value<'gc>, u8)>, Error<'gc>> {
    let mut depth = 0;
    let orig_proto = proto;

    while let Value::Object(p) = proto {
        // Surgical fix for Ruffle bug: Reduce recursion limit to prevent LoginServlet/StatusServlet
        // circular reference between sBaseURI and sBaseUri properties
        if depth == 10 {
            return Ok(None); // Return None instead of error to allow graceful fallback
        }

        if let Some(getter) = p.getter(name, activation)
            && let Some(exec) = getter.as_function()
        {
            let result = exec.exec(
                ExecutionName::Static("[Getter]"),
                activation,
                this.into(),
                1,
                &[],
                ExecutionReason::Special,
                getter,
            );

            match result {
                Err(Error::ThrownValue(e)) => return Err(Error::ThrownValue(e)),
                Err(Error::SpecialRecursionLimit) => {
                    // Fall back to local resolution for compatibility
                    // with SWF<7.
                }
                _ => {
                    let value = result.unwrap_or(Value::Undefined);
                    return Ok(Some((value, depth)));
                }
            };
        }

        if let Some(value) = p.get_local_stored(name, activation) {
            return Ok(Some((value, depth)));
        }

        proto = p.proto(activation);
        depth += 1;
    }

    if call_resolve_fn && let Some(resolve) = find_resolve_method(orig_proto, activation)? {
        let result = resolve.call(istr!("__resolve"), activation, this.into(), &[name.into()])?;
        return Ok(Some((result, 0)));
    }

    Ok(None)
}

/// Finds the appropriate `__resolve` method for an object, searching its hierarchy too.
pub fn find_resolve_method<'gc>(
    mut proto: Value<'gc>,
    activation: &mut Activation<'_, 'gc>,
) -> Result<Option<Object<'gc>>, Error<'gc>> {
    let mut depth = 0;

    while let Value::Object(p) = proto {
        if depth == 255 {
            return Err(Error::PrototypeRecursionLimit);
        }

        let resolve = p.get_local_stored(istr!("__resolve"), activation);
        // FP completely skips over primitives (but not over non-function objects).
        if let Some(Value::Object(value)) = resolve {
            return Ok(Some(value));
        }

        proto = p.proto(activation);
        depth += 1;
    }

    Ok(None)
}
