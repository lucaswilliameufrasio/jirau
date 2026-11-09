import XCTest
@testable import AllPlatformsShowcase

/// Testes do ViewModel contra o núcleo Rust real (AppEngine via UniFFI).
final class NativeLifecycleTests: XCTestCase {
    func testViewModelStartsFromRustState() {
        let vm = MainViewModel()
        XCTAssertEqual(vm.counter, 0)
        XCTAssertTrue(vm.status.contains("Rust"), "saudação deve vir do núcleo Rust: \(vm.status)")
    }

    func testStateTransitionsGoThroughRust() {
        let engine = AppEngine()
        let vm = MainViewModel(engine: engine)
        vm.increment()
        vm.increment()
        XCTAssertEqual(vm.counter, 2)
        XCTAssertEqual(engine.getState().counter, 2, "o estado vive no Rust")

        vm.decrement()
        XCTAssertEqual(vm.counter, 1)
    }

    func testRustRejectsNegativeCounter() {
        let vm = MainViewModel()
        vm.decrement()
        XCTAssertEqual(vm.counter, 0)
        XCTAssertTrue(vm.status.contains("negativo"))
    }
}
