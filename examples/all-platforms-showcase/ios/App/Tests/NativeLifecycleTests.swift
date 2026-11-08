import XCTest
@testable import AllPlatformsShowcase

final class NativeLifecycleTests: XCTestCase {
    func testViewModelInitialState() {
        let vm = MainViewModel()
        XCTAssertEqual(vm.counter, 0)
        XCTAssertTrue(vm.status.contains("conectado"))
    }

    func testViewModelStateTransitions() {
        let vm = MainViewModel()
        vm.increment()
        XCTAssertEqual(vm.counter, 1)

        vm.decrement()
        XCTAssertEqual(vm.counter, 0)

        // Limite inferior
        vm.decrement()
        XCTAssertEqual(vm.counter, 0)
        XCTAssertTrue(vm.status.contains("não pode ser negativo"))
    }
}
