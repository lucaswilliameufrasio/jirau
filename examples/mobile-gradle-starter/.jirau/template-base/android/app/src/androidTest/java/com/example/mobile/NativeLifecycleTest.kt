package com.example.mobile

import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** ViewModel e UI contra o núcleo Rust real (carregado via JNA/UniFFI). */
@RunWith(AndroidJUnit4::class)
class NativeLifecycleTest {
    @get:Rule
    val compose = createComposeRule()

    @Test
    fun greetingComesFromRust() {
        assertTrue(MainViewModel().status.value.contains("Android"))
    }

    @Test
    fun counterStateLivesInRust() {
        val vm = MainViewModel()
        assertEquals(0, vm.counter.value)
        vm.increment()
        vm.increment()
        assertEquals(2, vm.counter.value)
        vm.decrement()
        assertEquals(1, vm.counter.value)
    }

    @Test
    fun rustRejectsNegativeCounter() {
        val vm = MainViewModel()
        vm.decrement()
        assertEquals(0, vm.counter.value)
        assertTrue(vm.status.value.contains("não pode ficar negativo"))
    }

    @Test
    fun buttonsDriveTheRustEngine() {
        compose.setContent { AppScreen(MainViewModel()) }
        compose.onNodeWithTag("increment").performClick()
        compose.onNodeWithTag("increment").performClick()
        compose.onNodeWithTag("decrement").performClick()
        compose.onNodeWithTag("counter").assertTextEquals("1")
    }
}
