package com.example.showcase

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class NativeLifecycleTest {
    @Test
    fun testInitialStateAndCounter() {
        val vm = MainViewModel()
        assertEquals(0, vm.counter.value)
        assertTrue(vm.status.value.contains("conectado"))

        vm.increment()
        assertEquals(1, vm.counter.value)

        vm.decrement()
        assertEquals(0, vm.counter.value)

        // Limite inferior validado no Rust
        vm.decrement()
        assertEquals(0, vm.counter.value)
        assertTrue(vm.status.value.contains("não pode ser negativo"))
    }
}
