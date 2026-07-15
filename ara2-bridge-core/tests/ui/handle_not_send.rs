use ara2_bridge_core::Handle;

enum AudioSourceKind {}

fn require_send<T: Send>() {}

fn main() {
    require_send::<Handle<AudioSourceKind>>();
}
