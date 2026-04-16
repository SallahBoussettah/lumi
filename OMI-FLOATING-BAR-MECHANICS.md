# Omi Floating Control Bar: Implementation Mechanics

## Overview

The Omi floating control bar is a specialized macOS window that provides quick access to voice recording, AI chat, and push-to-talk functionality. It operates as a **non-activating panel** separate from the main app window, enabling seamless interaction without disrupting the user's focused workflow.

This document details the implementation mechanics—window creation, multi-screen handling, global hotkeys, voice recording, dragging, position persistence, and communication patterns—with code snippets for reference during Linux (Tauri) porting.

---

## 1. Window Creation & Configuration

### 1.1 NSPanel Subclass: FloatingControlBarWindow

The floating bar is implemented as a custom `NSPanel` subclass with non-activating behavior:

```swift
class FloatingControlBarWindow: NSPanel, NSWindowDelegate {
    override init(
        contentRect: NSRect, styleMask style: NSWindow.StyleMask,
        backing backingStoreType: NSWindow.BackingStoreType = .buffered, defer flag: Bool = false
    ) {
        let initialRect = NSRect(origin: .zero, size: FloatingControlBarWindow.minBarSize)

        super.init(
            contentRect: initialRect,
            styleMask: [.borderless, .nonactivatingPanel],  // ← Key: non-activating
            backing: backingStoreType,
            defer: flag
        )

        self.appearance = NSAppearance(named: .vibrantDark)
        self.isOpaque = false
        self.backgroundColor = .clear
        self.hasShadow = false
        self.level = .floating  // ← NSWindowLevel.floating (stays above normal windows)
        self.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        self.isMovableByWindowBackground = false  // Custom drag handler instead
        self.delegate = self
        self.minSize = FloatingControlBarWindow.minBarSize
        self.maxSize = FloatingControlBarWindow.maxBarSize
    }

    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { false }
}
```

**Key Window Properties:**

| Property | Value | Purpose |
|----------|-------|---------|
| `styleMask` | `.borderless, .nonactivatingPanel` | No title bar; doesn't activate main window when clicked |
| `level` | `.floating` | NSWindowLevel.floating = 4; stays above regular windows |
| `collectionBehavior` | `.canJoinAllSpaces, .fullScreenAuxiliary` | Visible on all desktops & fullscreen apps |
| `isOpaque` | `false` | Transparent background for rounded corners & shadows |
| `backgroundColor` | `.clear` | No opaque fill |
| `hasShadow` | `false` | Custom background via FloatingBackgroundModifier |

### 1.2 SwiftUI Content Wrapping

The SwiftUI view is wrapped in a **container pattern** to avoid constraint update loops:

```swift
private func setupViews() {
    let swiftUIView = FloatingControlBarView(
        window: self,
        onPlayPause: { [weak self] in self?.onPlayPause?() },
        onAskAI: { [weak self] in self?.handleAskAI() },
        // ... other callbacks
    ).environmentObject(state)

    hostingView = FloatingBarHostingView(rootView: AnyView(
        swiftUIView
            .withFontScaling()
            .preferredColorScheme(.dark)
            .environment(\.colorScheme, .dark)
    ))
    hostingView?.appearance = NSAppearance(named: .vibrantDark)

    // CRITICAL: Use a container view instead of NSHostingView as contentView.
    // When NSHostingView IS the contentView, it tries to negotiate window sizing,
    // causing re-entrant constraint updates that crash on macOS 26+.
    let container = NSView()
    self.contentView = container

    if let hosting = hostingView {
        hosting.sizingOptions = [.minSize, .maxSize]  // Exclude .intrinsicContentSize
        hosting.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(hosting)
        NSLayoutConstraint.activate([
            hosting.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            hosting.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            hosting.topAnchor.constraint(equalTo: container.topAnchor),
            hosting.bottomAnchor.constraint(equalTo: container.bottomAnchor),
        ])
    }
}
```

**Why the container pattern?**
- Direct NSHostingView as contentView triggers `updateAnimatedWindowSize` → constraint updates → loop
- Wrapping breaks the "I own the window" relationship, allowing safe resizing

### 1.3 Lifecycle: Setup & Creation

The floating bar window is created **lazily after onboarding**:

```swift
// In DesktopHomeView.swift (main content appears):
FloatingControlBarManager.shared.setup(
    appState: appState, 
    chatProvider: viewModelContainer.chatProvider
)
if FloatingControlBarManager.shared.isEnabled {
    FloatingControlBarManager.shared.show()
}
```

Setup method (from FloatingControlBarManager):

```swift
@MainActor
func setup(appState: AppState, chatProvider: ChatProvider) {
    guard window == nil else {
        log("FloatingControlBarManager: setup() called but window already exists")
        return
    }
    
    let barWindow = FloatingControlBarWindow(
        contentRect: .zero,
        styleMask: [.borderless],
        backing: .buffered,
        defer: false
    )
    
    // Wire up callbacks and state observers
    barWindow.onPlayPause = { [weak appState] in
        guard let appState = appState else { return }
        appState.toggleTranscription()
    }
    
    barWindow.onAskAI = { [weak barWindow] in
        barWindow?.showAIConversation()
        barWindow?.makeKeyAndOrderFront(nil)
    }
    
    barWindow.onHide = { [weak self] in
        self?.isEnabled = false
    }
    
    self.window = barWindow
}
```

**Visibility Control:**

```swift
var isEnabled: Bool {
    get {
        if UserDefaults.standard.object(forKey: Self.kAskOmiEnabled) == nil {
            return true  // Default to enabled for new users
        }
        return UserDefaults.standard.bool(forKey: Self.kAskOmiEnabled)
    }
    set {
        UserDefaults.standard.set(newValue, forKey: Self.kAskOmiEnabled)
    }
}

func show() {
    isEnabled = true
    window?.makeKeyAndOrderFront(nil)
}

func hide() {
    isEnabled = false
    window?.orderOut(nil)
}
```

---

## 2. Multi-Screen Handling

The floating bar automatically moves with the cursor across monitors and revalidates position on screen changes.

### 2.1 Cursor-Following (250ms Polling)

```swift
private func startCursorScreenTracking() {
    let timer = DispatchSource.makeTimerSource(queue: .main)
    timer.schedule(deadline: .now(), repeating: .milliseconds(250))
    timer.setEventHandler { [weak self] in
        self?.checkCursorScreen()
    }
    timer.resume()
    cursorTrackingTimer = timer
}

private func checkCursorScreen() {
    guard NSScreen.screens.count > 1 else { return }

    let mouseLocation = NSEvent.mouseLocation
    guard let targetScreen = NSScreen.screens.first(where: { $0.frame.contains(mouseLocation) }) else { return }

    let currentScreen = self.screen ?? NSScreen.main
    if targetScreen == currentScreen { return }

    let currentVisible = currentScreen?.visibleFrame ?? .zero
    let targetVisible = targetScreen.visibleFrame

    if ShortcutSettings.shared.draggableBarEnabled {
        // Translate position proportionally across screens
        let relX = currentVisible.width > 0 ? (frame.origin.x - currentVisible.origin.x) / currentVisible.width : 0.5
        let relY = currentVisible.height > 0 ? (frame.origin.y - currentVisible.origin.y) / currentVisible.height : 1.0
        let newX = targetVisible.origin.x + relX * targetVisible.width
        let newY = targetVisible.origin.y + relY * targetVisible.height
        setFrameOrigin(NSPoint(x: newX, y: newY))
        UserDefaults.standard.set(NSStringFromPoint(frame.origin), forKey: FloatingControlBarWindow.positionKey)
    } else {
        // Non-draggable: center on new screen
        let x = targetVisible.midX - frame.width / 2
        let y = targetVisible.maxY - frame.height - 20
        setFrameOrigin(NSPoint(x: x, y: y))
    }

    log("FloatingControlBarWindow: followed cursor to screen \(targetScreen.localizedName)")
}
```

### 2.2 Screen Validation (on Connect/Disconnect)

```swift
// Observes NSApplication.didChangeScreenParametersNotification
private func validatePositionOnScreenChange() {
    if !ShortcutSettings.shared.draggableBarEnabled {
        // Non-draggable: always restore to default position
        centerOnMainScreen()
        return
    }

    let barFrame = self.frame
    let center = NSPoint(x: barFrame.midX, y: barFrame.midY)
    let onScreen = NSScreen.screens.contains { $0.visibleFrame.contains(center) }
    if !onScreen {
        log("FloatingControlBarWindow: bar center off-screen after monitor change, re-centering")
        UserDefaults.standard.removeObject(forKey: FloatingControlBarWindow.positionKey)
        centerOnMainScreen()
    }
}

private func centerOnMainScreen() {
    let targetScreen = NSApp.keyWindow?.screen ?? NSScreen.main ?? NSScreen.screens.first
    guard let screen = targetScreen else {
        self.center()
        return
    }
    let visibleFrame = screen.visibleFrame
    let x = visibleFrame.midX - frame.width / 2
    let y = visibleFrame.maxY - frame.height - 20
    self.setFrameOrigin(NSPoint(x: x, y: y))
}
```

**Gotchas for Linux (Wayland/X11):**
- **Wayland**: No reliable way to get absolute mouse position across monitors; use DBus `org.freedesktop.ScreenSaver` for active monitor detection
- **X11**: Use `XRRGetScreenResourcesCurrent()` to enumerate monitors, poll `XQueryPointer()` for cursor location (same 250ms interval)
- **Window move**: On Wayland, compositor controls window positioning; apps can only request via xdg_toplevel move/resize events

---

## 3. Global Hotkey System

The floating bar responds to two global shortcuts: **"Ask Omi"** (toggles input panel) and **"Push to Talk"** (voice recording).

### 3.1 Ask Omi Shortcut (Carbon HotKeys API)

The "Ask Omi" shortcut uses the Carbon `RegisterEventHotKey` API for truly global keyboard interception:

```swift
class GlobalShortcutManager {
    static let shared = GlobalShortcutManager()

    private var hotKeyRefs: [HotKeyID: EventHotKeyRef] = [:]
    private enum HotKeyID: UInt32 {
        case askOmi = 2
    }

    private init() {
        var eventType = EventTypeSpec(
            eventClass: OSType(kEventClassKeyboard),
            eventKind: OSType(kEventHotKeyPressed)
        )
        InstallEventHandler(
            GetApplicationEventTarget(),
            { (_, event, _) -> OSStatus in
                return GlobalShortcutManager.shared.handleHotKeyEvent(event!)
            },
            1, &eventType, nil, nil
        )

        // Re-register when user changes shortcut in settings
        shortcutObserver = NotificationCenter.default.addObserver(
            forName: ShortcutSettings.askOmiShortcutChanged,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.registerAskOmi()
        }
    }

    private func registerAskOmi() {
        guard !isRegistrationSuspended else { return }
        if let ref = hotKeyRefs.removeValue(forKey: .askOmi) {
            UnregisterEventHotKey(ref)  // Unregister old hotkey
        }

        let (askOmiEnabled, askOmiShortcut) = MainActor.assumeIsolated {
            (ShortcutSettings.shared.askOmiEnabled, ShortcutSettings.shared.askOmiShortcut)
        }
        guard askOmiEnabled else { return }
        guard askOmiShortcut.supportsGlobalHotKey, let keyCode = askOmiShortcut.keyCode else { return }

        registerHotKey(keyCode: Int(keyCode), modifiers: askOmiShortcut.carbonModifiers, id: .askOmi)
        NSLog("GlobalShortcutManager: Registered Ask Omi shortcut: \(askOmiShortcut.displayLabel)")
    }

    private func registerHotKey(keyCode: Int, modifiers: Int, id: HotKeyID) {
        var hotKeyRef: EventHotKeyRef?
        let hotKeyID = EventHotKeyID(signature: FourCharCode(0x4F4D4921), id: id.rawValue) // "OMI!"

        let status = RegisterEventHotKey(
            UInt32(keyCode), 
            UInt32(modifiers), 
            hotKeyID,
            GetApplicationEventTarget(), 
            0, 
            &hotKeyRef
        )

        if status == noErr, let ref = hotKeyRef {
            hotKeyRefs[id] = ref
        } else {
            NSLog("GlobalShortcutManager: Failed to register hotkey, error: \(status)")
        }
    }

    private func handleHotKeyEvent(_ event: EventRef) -> OSStatus {
        var hotKeyID = EventHotKeyID()
        let status = GetEventParameter(
            event,
            OSType(kEventParamDirectObject),
            OSType(typeEventHotKeyID),
            nil,
            MemoryLayout<EventHotKeyID>.size,
            nil,
            &hotKeyID
        )

        guard status == noErr, let id = HotKeyID(rawValue: hotKeyID.id) else { return status }

        switch id {
        case .askOmi:
            NSLog("GlobalShortcutManager: Ask Omi shortcut detected")
            DispatchQueue.main.async {
                FloatingControlBarManager.shared.toggleAIInput()
            }
        }

        return noErr
    }

    func unregisterShortcuts() {
        for (_, ref) in hotKeyRefs {
            UnregisterEventHotKey(ref)
        }
        hotKeyRefs.removeAll()
    }
}
```

**Shortcut Storage:**

```swift
struct KeyboardShortcut: Codable, Hashable {
    var keyCode: UInt16?
    var keyDisplay: String?
    var modifiersRawValue: UInt
    var modifierOnly: Bool
    var requiresRightCommand: Bool

    var carbonModifiers: Int {
        var value = 0
        if modifiers.contains(.command) { value |= Int(cmdKey) }
        if modifiers.contains(.shift) { value |= Int(shiftKey) }
        if modifiers.contains(.option) { value |= Int(optionKey) }
        if modifiers.contains(.control) { value |= Int(controlKey) }
        if modifiers.contains(.function) { value |= Int(kEventKeyModifierFnMask) }
        return value
    }
}
```

**Linux Equivalent (Tauri + DBus):**
- Use `zbus` crate for DBus org.gnome.Shell keybindings (`GSettings schema: org.gnome.Shell.keybindings`)
- Register via `media-keys` interface: `org.gnome.Shell.introspection` (GNOME 42+)
- Fall back to X11 `XGrabKey()` for unsupported WMs (KDE, XFCE, i3)
- Wayland: Not standardized; requires compositor plugin (GNOME has Shell extensions, KDE has KGlobalShortcuts)

### 3.2 Push-to-Talk (PTT) Shortcut (NSEvent Monitors)

PTT uses local and global NSEvent monitors for finer-grained control:

```swift
@MainActor
class PushToTalkManager: ObservableObject {
    static let shared = PushToTalkManager()

    enum PTTState {
        case idle
        case listening
        case pendingLockDecision
        case lockedListening
        case finalizing
    }

    @Published private(set) var state: PTTState = .idle

    private var globalMonitor: Any?
    private var localMonitor: Any?

    private func installEventMonitors() {
        removeEventMonitors()

        let monitorMask: NSEvent.EventTypeMask = [.flagsChanged, .keyDown, .keyUp]

        // Global monitor — fires when OTHER apps are focused
        globalMonitor = NSEvent.addGlobalMonitorForEvents(matching: monitorMask) {
            [weak self] event in
            Task { @MainActor in
                self?.handleShortcutEvent(event)
            }
        }

        // Local monitor — fires when THIS app is focused
        localMonitor = NSEvent.addLocalMonitorForEvents(matching: monitorMask) { [weak self] event in
            Task { @MainActor in
                self?.handleShortcutEvent(event)
            }
            return event
        }

        log("PushToTalkManager: event monitors installed")
    }

    private func handleShortcutEvent(_ event: NSEvent) {
        guard ShortcutSettings.shared.pttEnabled else { return }
        let shortcut = ShortcutSettings.shared.pttShortcut

        let pttActive: Bool
        switch event.type {
        case .flagsChanged:
            guard shortcut.modifierOnly else { return }
            pttActive = shortcut.matchesFlagsChanged(event)
        case .keyDown:
            guard !shortcut.modifierOnly, !event.isARepeat else { return }
            pttActive = shortcut.matchesKeyDown(event)
        case .keyUp:
            guard !shortcut.modifierOnly else { return }
            pttActive = false
            if shortcut.matchesKeyUp(event) {
                handleShortcutUp()
            }
            return
        default:
            return
        }

        // Auto-show bar if PTT is pressed (even if bar is hidden)
        if pttActive, !FloatingControlBarManager.shared.isVisible {
            FloatingControlBarManager.shared.show()
        }

        guard FloatingControlBarManager.shared.isVisible else { return }

        if pttActive {
            handleShortcutDown()
        } else if shortcut.modifierOnly {
            handleShortcutUp()
        }
    }
}
```

---

## 4. Push-to-Talk (PTT) Voice Recording

The PTT system captures voice while a modifier key (typically ⌥ Option) is held, with support for both **hold mode** and **locked mode** (double-tap to lock).

### 4.1 PTT State Machine

```swift
// State transitions:
//   idle → [Option down] → listening → [Option up] → finalizing → sends query → idle
//   idle → [Quick tap] → pendingLockDecision → [tap again <400ms] → lockedListening
//   pendingLockDecision → [timeout] → finalizing → sends query → idle

private func handleShortcutDown() {
    let now = ProcessInfo.processInfo.systemUptime

    switch state {
    case .idle:
        // Check for double-tap: if last Option-up was recent, enter locked mode
        if ShortcutSettings.shared.doubleTapForLock && (now - lastOptionUpTime) < doubleTapThreshold {
            lastOptionUpTime = 0
            enterLockedListening()
        } else {
            lastOptionDownTime = now
            startListening()
        }

    case .listening:
        // Already listening (hold mode), ignore repeated flagsChanged
        break

    case .pendingLockDecision:
        // Tap while pending → finalize quickly and enter locked mode
        stopListening()
        enterLockedListening()

    case .lockedListening:
        // Tap while locked → finalize and send
        finalize()

    case .finalizing:
        break
    }
}

private func handleShortcutUp() {
    let now = ProcessInfo.processInfo.systemUptime

    switch state {
    case .listening:
        let holdDuration = now - lastOptionDownTime

        if ShortcutSettings.shared.doubleTapForLock && holdDuration < tapToLockMaxHoldDuration {
            lastOptionUpTime = now
            enterPendingLockDecision()  // Wait for second tap
        } else {
            lastOptionUpTime = 0
            finalize()  // Long hold → finalize immediately
        }

    case .pendingLockDecision, .lockedListening, .idle, .finalizing:
        break
    }
}
```

### 4.2 Audio Capture & Transcription

The PTT system supports two transcription modes:

#### **Batch Mode** (post-recording transcription):
```swift
private func finalize() {
    guard state == .listening || state == .lockedListening || state == .pendingLockDecision else { return }

    state = .finalizing
    audioCaptureService?.stopCapture()

    // Play end-of-PTT sound
    if ShortcutSettings.shared.pttSoundsEnabled {
        let sound = NSSound(named: "Bottle")
        sound?.volume = 0.3
        sound?.play()
    }

    let isBatchMode = ShortcutSettings.shared.pttTranscriptionMode == .batch

    if isBatchMode {
        // Batch mode: send accumulated audio buffer to post-recorded API
        log("PushToTalkManager: finalizing (batch) — mic stopped, transcribing recorded audio")
        batchAudioLock.lock()
        let audioData = batchAudioBuffer
        batchAudioBuffer = Data()
        batchAudioLock.unlock()

        stopAudioTranscription()

        guard !audioData.isEmpty else {
            log("PushToTalkManager: batch mode — no audio recorded")
            sendTranscript()
            return
        }

        barState?.voiceTranscript = "Transcribing..."

        Task {
            do {
                let language = AssistantSettings.shared.effectiveTranscriptionLanguage
                let audioSeconds = Double(audioData.count) / (16000.0 * 2.0)
                log("PushToTalkManager: batch audio \(audioData.count) bytes (\(String(format: "%.1f", audioSeconds))s)")

                // First attempt with user's effective language (usually "multi")
                var transcript = try await TranscriptionService.batchTranscribe(
                    audioData: audioData,
                    language: language
                )

                // If multi-language detection returned empty on short audio (<5s),
                // retry with user's explicit language
                if (transcript == nil || transcript?.isEmpty == true)
                    && language == "multi" && audioSeconds < 5.0 {
                    let fallback = AssistantSettings.shared.transcriptionLanguage
                    let retryLang = (fallback.isEmpty || fallback == "multi") ? "en" : fallback
                    log("PushToTalkManager: multi returned empty, retrying with '\(retryLang)'")
                    transcript = try await TranscriptionService.batchTranscribe(
                        audioData: audioData,
                        language: retryLang
                    )
                }

                if let transcript, !transcript.isEmpty {
                    self.transcriptSegments = [transcript]
                } else {
                    log("PushToTalkManager: transcription returned empty after retry")
                }
            } catch {
                logError("PushToTalkManager: batch transcription failed", error: error)
                let message = (error as? TranscriptionService.TranscriptionError)?.errorDescription ?? "Transcription failed"
                barState?.voiceTranscript = "⚠️ \(message)"
                try? await Task.sleep(nanoseconds: 3_000_000_000)
                barState?.voiceTranscript = ""
            }
            self.sendTranscript()
        }
    }
}
```

#### **Live Mode** (streaming transcription):
```swift
private func startAudioTranscription() {
    hasMicPermission = AudioCaptureService.checkPermission()

    guard hasMicPermission else {
        log("PushToTalkManager: no microphone permission, requesting")
        Task {
            let granted = await AudioCaptureService.requestPermission()
            self.hasMicPermission = granted
            if granted {
                log("PushToTalkManager: microphone permission granted")
            } else {
                log("PushToTalkManager: microphone permission denied")
                self.stopListening()
            }
        }
        return
    }

    let isBatchMode = ShortcutSettings.shared.pttTranscriptionMode == .batch

    if isBatchMode {
        batchAudioLock.lock()
        batchAudioBuffer = Data()
        batchAudioLock.unlock()
        startMicCapture(batchMode: true)
        log("PushToTalkManager: started audio capture (batch mode)")
    } else {
        // Live mode: start mic capture and stream to Deepgram
        startMicCapture()

        do {
            let language = AssistantSettings.shared.effectiveTranscriptionLanguage
            let service = try TranscriptionService(language: language, channels: 1)
            transcriptionService = service

            service.start(
                onSegments: { [weak self] segments in
                    Task { @MainActor in
                        self?.handleTranscriptSegments(segments)
                    }
                },
                onEvent: { _ in },
                onError: { [weak self] error in
                    Task { @MainActor in
                        logError("PushToTalkManager: transcription error", error: error)
                        self?.stopListening()
                    }
                },
                onConnected: {
                    Task { @MainActor in
                        log("PushToTalkManager: backend connected")
                    }
                }
            )
        } catch {
            logError("PushToTalkManager: failed to create TranscriptionService", error: error)
            stopListening()
        }
    }
}
```

### 4.3 Mic Fallback (Silent Mic Watchdog)

Bluetooth input sometimes returns zero samples due to A2DP profile conflicts. The PTT system detects this and falls back to the built-in mic:

```swift
private func startMicCapture(batchMode: Bool = false, overrideDeviceID: AudioDeviceID? = nil) {
    if audioCaptureService == nil {
        if let override = overrideDeviceID {
            audioCaptureService = AudioCaptureService(overrideDeviceID: override)
        } else {
            audioCaptureService = AudioCaptureService()
        }
    }
    guard let capture = audioCaptureService else { return }

    // Silent-mic watchdog: Bluetooth input often returns zero samples while another app
    // holds A2DP output. Fall back to the built-in mic so PTT still captures the user.
    capture.onSilentMicDetected = { [weak self] in
        Task { @MainActor in
            self?.handleSilentMicFallback(batchMode: batchMode)
        }
    }

    Task { @MainActor [weak self] in
        guard let self else { return }
        do {
            try await capture.startCapture(
                onAudioChunk: { [weak self] audioData in
                    guard let self else { return }
                    if batchMode {
                        self.batchAudioLock.lock()
                        self.batchAudioBuffer.append(audioData)
                        self.batchAudioLock.unlock()
                    } else {
                        self.transcriptionService?.sendAudio(audioData)
                    }
                },
                onAudioLevel: { _ in }
            )
            log("PushToTalkManager: mic capture started (batch=\(batchMode))")
        } catch {
            logError("PushToTalkManager: mic capture failed", error: error)
            self.stopListening()
        }
    }
}

private func handleSilentMicFallback(batchMode: Bool) {
    guard state == .listening || state == .lockedListening || state == .pendingLockDecision else {
        return
    }
    guard let builtInID = AudioCaptureService.findBuiltInMicDeviceID() else {
        log("PushToTalkManager: silent-mic detected but no built-in mic to fall back to")
        return
    }
    log("PushToTalkManager: silent-mic fallback — switching to built-in mic")
    audioCaptureService?.stopCapture()
    audioCaptureService = nil
    startMicCapture(batchMode: batchMode, overrideDeviceID: builtInID)
}
```

---

## 5. Hover-to-Expand

The compact pill (40×14) expands to a larger widget (210×50) on hover, triggered by the SwiftUI `onHover()` modifier.

### 5.1 SwiftUI Hover Detection

```swift
// In FloatingControlBarView
private var barChrome: some View {
    VStack(spacing: 0) {
        controlBarView
        if state.showingAIConversation {
            conversationView
        }
    }
    .frame(maxWidth: barNeedsFullWidth ? .infinity : nil, alignment: .top)
    .overlay(alignment: .topTrailing) {
        if isHovering && !state.isVoiceListening {
            Button { openFloatingBarSettings() } label: {
                Image(systemName: "gearshape.fill")
                    .font(.system(size: 11))
                    .foregroundColor(.white.opacity(0.7))
                    .frame(width: 22, height: 22)
                    .background(Color.white.opacity(0.12))
                    .cornerRadius(5)
            }
            .buttonStyle(.plain)
            .padding(6)
            .transition(.opacity)
        }
    }
    .clipped()
    .background(DraggableAreaView(targetWindow: window))
    .floatingBackground(cornerRadius: barNeedsFullWidth ? 20 : 5)
    .onHover(perform: handleBarHover)  // ← Hover detection
}

private func handleBarHover(_ hovering: Bool) {
    if !hovering {
        state.requiresHoverReset = false
    }

    let effectiveHover = hovering && !state.requiresHoverReset
    state.isHoveringBar = effectiveHover
    
    // Resize window BEFORE updating SwiftUI state on expand
    if effectiveHover {
        (window as? FloatingControlBarWindow)?.resizeForHover(expanded: true)
    }
    withAnimation(.easeInOut(duration: 0.2)) {
        isHovering = effectiveHover
    }
    if !effectiveHover {
        (window as? FloatingControlBarWindow)?.resizeForHover(expanded: false)
    }
}
```

### 5.2 Window Resize (Center-Anchored)

```swift
func resizeForHover(expanded: Bool) {
    guard !state.showingAIConversation, !state.isVoiceListening, !state.isShowingNotification, !suppressHoverResize else { return }
    resizeWorkItem?.cancel()
    resizeWorkItem = nil

    let targetSize = expanded ? FloatingControlBarWindow.expandedBarSize : FloatingControlBarWindow.minBarSize

    let doResize: () -> Void = { [weak self] in
        guard let self = self else { return }
        guard !self.state.showingAIConversation,
              !self.state.isVoiceListening,
              !self.state.isShowingNotification,
              !self.suppressHoverResize
        else { return }
        
        // Center-anchor: grows outward from current center
        let newOrigin = NSPoint(
            x: self.frame.midX - targetSize.width / 2,
            y: self.frame.midY - targetSize.height / 2
        )
        self.styleMask.remove(.resizable)
        self.isResizingProgrammatically = true
        self.setFrame(NSRect(origin: newOrigin, size: targetSize), display: true, animate: false)
        self.isResizingProgrammatically = false
    }

    if expanded {
        // Expand synchronously so content renders in the correct size
        doResize()
    } else {
        // Collapse async to avoid blocking SwiftUI evaluation
        resizeWorkItem = DispatchWorkItem(block: doResize)
        DispatchQueue.main.async(execute: resizeWorkItem!)
    }
}
```

**Key Sizes:**

| State | Width | Height | Notes |
|-------|-------|--------|-------|
| Compact pill | 40 | 14 | Minimal bar, not hovering |
| Expanded pill | 210 | 50 | Hovering; shows buttons + shortcuts |
| AI chat | 430 | ~250+ | Input panel; auto-expands with content |
| Notification | 430 | ~108 | Notification + bar combined |

---

## 6. Position & Size Persistence

The window's position and size are saved to `UserDefaults` and restored on launch.

### 6.1 Position Storage

```swift
private static let positionKey = "FloatingControlBarPosition"
private static let sizeKey = "FloatingControlBarSize"

// On init:
if ShortcutSettings.shared.draggableBarEnabled,
   let savedPosition = UserDefaults.standard.string(forKey: FloatingControlBarWindow.positionKey) {
    let origin = NSPointFromString(savedPosition)
    // Verify saved position is on a visible screen
    let onScreen = NSScreen.screens.contains { $0.visibleFrame.contains(NSPoint(x: origin.x + 14, y: origin.y + 14)) }
    if onScreen {
        self.setFrameOrigin(origin)
    } else {
        centerOnMainScreen()
    }
} else {
    centerOnMainScreen()
}

// Persist position (NSWindowDelegate):
@objc func windowDidMove(_ notification: Notification) {
    // Only persist when user is physically dragging (not programmatic moves)
    guard isUserDragging else { return }
    UserDefaults.standard.set(
        NSStringFromPoint(self.frame.origin), forKey: FloatingControlBarWindow.positionKey
    )
}

// Persist size when user manually resizes the AI response window:
func windowDidResize(_ notification: Notification) {
    if !isResizingProgrammatically && !isUserResizing && state.showingAIResponse {
        UserDefaults.standard.set(
            NSStringFromSize(self.frame.size), forKey: FloatingControlBarWindow.sizeKey
        )
    }
}
```

### 6.2 Size Restoration (AI Response Height Capping)

When the AI response is shown, the window's height is capped at **2× the user's preferred height**:

```swift
private func resizeToResponseHeight(animated: Bool = false) {
    // Determine the 2× cap from the user's saved (or default) preferred height.
    let savedSize = UserDefaults.standard.string(forKey: FloatingControlBarWindow.sizeKey)
        .map(NSSizeFromString)
    let baseHeight = savedSize.map { max($0.height, Self.defaultBaseResponseHeight) } ?? Self.defaultBaseResponseHeight
    let maxHeight = baseHeight * 2

    let startHeight = max(Self.minResponseHeight, frame.height)
    let initialSize = NSSize(width: Self.expandedWidth, height: startHeight)
    resizeAnchored(to: initialSize, makeResizable: true, animated: animated, anchorTop: true)
    setupResponseHeightObserver(maxHeight: maxHeight)
}

private func setupResponseHeightObserver(maxHeight: CGFloat) {
    responseHeightCancellable?.cancel()
    responseHeightCancellable = state.$responseContentHeight
        .removeDuplicates()
        .debounce(for: .milliseconds(80), scheduler: DispatchQueue.main)
        .sink { [weak self] contentHeight in
            guard let self = self,
                  self.state.showingAIResponse,
                  !self.isUserResizing,
                  contentHeight > 0
            else { return }
            let targetHeight = (contentHeight + Self.responseViewOverhead).rounded()
            let clampedHeight = min(max(targetHeight, Self.minResponseHeight), maxHeight)
            // Only expand, never auto-shrink.
            guard clampedHeight > self.frame.height + 2 else { return }
            self.resizeAnchored(
                to: NSSize(width: Self.expandedWidth, height: clampedHeight),
                makeResizable: true,
                animated: true,
                anchorTop: true
            )
        }
}
```

---

## 7. Dragging

The user can drag the bar around when **Draggable Bar** is enabled in settings.

### 7.1 Drag Handling (NSView Subclass)

```swift
struct DraggableAreaView: NSViewRepresentable {
    let targetWindow: NSWindow?

    func makeNSView(context: Context) -> NSView {
        let view = DraggableNSView()
        view.targetWindow = targetWindow
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {}

    class DraggableNSView: NSView {
        weak var targetWindow: NSWindow?
        private var initialLocation: NSPoint?

        override func mouseDown(with event: NSEvent) {
            guard ShortcutSettings.shared.draggableBarEnabled else {
                super.mouseDown(with: event)
                return
            }
            initialLocation = event.locationInWindow
            NotificationCenter.default.post(name: .floatingBarDragDidStart, object: nil)
        }

        override func mouseUp(with event: NSEvent) {
            guard initialLocation != nil else {
                super.mouseUp(with: event)
                return
            }
            super.mouseUp(with: event)
            NotificationCenter.default.post(name: .floatingBarDragDidEnd, object: nil)
            initialLocation = nil
        }

        override func mouseDragged(with event: NSEvent) {
            guard ShortcutSettings.shared.draggableBarEnabled,
                  let targetWindow = targetWindow, 
                  let initialLocation = initialLocation 
            else {
                return
            }

            let currentLocation = event.locationInWindow
            let newOrigin = NSPoint(
                x: targetWindow.frame.origin.x + (currentLocation.x - initialLocation.x),
                y: targetWindow.frame.origin.y + (currentLocation.y - initialLocation.y)
            )

            NSAnimationContext.beginGrouping()
            NSAnimationContext.current.duration = 0
            targetWindow.setFrameOrigin(newOrigin)
            NSAnimationContext.endGrouping()
        }
    }
}
```

### 7.2 Drag State Tracking

```swift
// In FloatingControlBarWindow.setupViews():
NotificationCenter.default.addObserver(
    forName: .floatingBarDragDidStart, object: nil, queue: .main
) { [weak self] _ in
    Task { @MainActor in
        self?.isUserDragging = true
        self?.state.isDragging = true
    }
}

NotificationCenter.default.addObserver(
    forName: .floatingBarDragDidEnd, object: nil, queue: .main
) { [weak self] _ in
    Task { @MainActor in
        self?.isUserDragging = false
        self?.state.isDragging = false
    }
}
```

---

## 8. Main App Communication

The floating bar exchanges data with the main app and backend via shared state objects and a dedicated ChatProvider.

### 8.1 Shared State (FloatingControlBarState)

```swift
@MainActor
class FloatingControlBarState: NSObject, ObservableObject {
    // Recording
    @Published var isRecording: Bool = false
    @Published var duration: Int = 0

    // AI conversation
    @Published var showingAIConversation: Bool = false
    @Published var showingAIResponse: Bool = false
    @Published var isAILoading: Bool = true
    @Published var aiInputText: String = ""
    @Published var currentAIMessage: ChatMessage? = nil
    @Published var displayedQuery: String = ""
    @Published var chatHistory: [FloatingChatExchange] = []

    // Voice/PTT
    @Published var isVoiceListening: Bool = false
    @Published var isVoiceLocked: Bool = false
    @Published var voiceTranscript: String = ""
    @Published var isVoiceFollowUp: Bool = false
    @Published var voiceFollowUpTranscript: String = ""

    // Notifications
    @Published var currentNotification: FloatingBarNotification? = nil
}
```

### 8.2 Isolated ChatProvider for Floating Bar

The floating bar uses its own `ChatProvider` instance to avoid interfering with the main chat history:

```swift
func setup(appState: AppState, chatProvider: ChatProvider) {
    // Keep the shared provider for syncing persisted messages into the main chat history
    historyChatProvider = chatProvider
    
    // Create an isolated provider for floating-bar sends only
    let floatingProvider = floatingChatProvider ?? ChatProvider()
    floatingProvider.modelOverride = chatProvider.modelOverride
    floatingProvider.workingDirectory = chatProvider.workingDirectory
    floatingChatProvider = floatingProvider

    barWindow.onSendQuery = { [weak self, weak barWindow, weak floatingProvider] message in
        guard let self = self, let barWindow = barWindow, let provider = floatingProvider else { return }
        Task { @MainActor in
            await self.sendAIQuery(message, barWindow: barWindow, provider: provider)
        }
    }
}
```

### 8.3 AI Query Sending

```swift
private func sendAIQuery(_ message: String, barWindow: FloatingControlBarWindow, provider: ChatProvider) async {
    let generation = activeQueryGeneration + 1
    activeQueryGeneration = generation

    guard let state = barWindow.state else { return }

    state.displayedQuery = message
    state.isAILoading = true
    state.showingAIResponse = false

    markConversationActivity()
    AnalyticsManager.shared.floatingBarQuerySent(
        model: state.selectedModel,
        fromVoice: state.currentQueryFromVoice
    )

    // If we're in follow-up mode (response window already open),
    // append the message to chat history instead of creating a new session
    let isFollowUp = state.showingAIConversation && state.showingAIResponse
    let exchange = FloatingChatExchange(
        question: message,
        questionMessageId: nil,
        aiMessage: ChatMessage(text: "", sender: .ai)
    )

    if isFollowUp {
        state.chatHistory.append(exchange)
    } else {
        state.chatHistory = [exchange]
    }

    var chunkCount = 0

    chatCancellable = provider.sendMessage(
        message,
        inConversation: floatingSessionKey,
        workingDirectory: nil
    )
    .sink(
        receiveCompletion: { [weak self, weak barWindow] completion in
            guard let self = self, self.activeQueryGeneration == generation else { return }
            switch completion {
            case .failure(let error):
                log("FloatingBar: query failed: \(error)")
                barWindow?.updateAIResponse(type: "error", text: "\(error)")
                self.activeQueryGeneration += 1
            case .finished:
                log("FloatingBar: query finished (\(chunkCount) chunks)")
                self.activeQueryGeneration += 1
            }
        },
        receiveValue: { [weak self, weak barWindow] chatMessage in
            guard let self = self, self.activeQueryGeneration == generation else { return }
            chunkCount += 1
            barWindow?.updateAIResponse(type: "data", text: chatMessage.text)
        }
    )
}
```

### 8.4 Recording State Sync

The floating bar observes the main app's recording state:

```swift
// In FloatingControlBarManager.setup():
recordingCancellable = appState.$isTranscribing
    .combineLatest(appState.$isSavingConversation)
    .receive(on: DispatchQueue.main)
    .sink { [weak barWindow] isTranscribing, isSaving in
        barWindow?.updateRecordingState(
            isRecording: isTranscribing,
            duration: Int(RecordingTimer.shared.duration),
            isInitialising: isSaving
        )
    }

durationCancellable = RecordingTimer.shared.$duration
    .receive(on: DispatchQueue.main)
    .sink { [weak barWindow, weak appState] duration in
        guard let appState = appState else { return }
        barWindow?.updateRecordingState(
            isRecording: appState.isTranscribing,
            duration: Int(duration),
            isInitialising: appState.isSavingConversation
        )
    }
```

---

## 9. Window Lifecycle

The floating bar window is created once during app initialization and remains alive until app termination. It can be hidden/shown but is never destroyed.

### 9.1 Creation

```swift
// In DesktopHomeView (after onboarding):
FloatingControlBarManager.shared.setup(
    appState: appState, 
    chatProvider: viewModelContainer.chatProvider
)
if FloatingControlBarManager.shared.isEnabled {
    FloatingControlBarManager.shared.show()
}
```

### 9.2 Visibility Lifecycle

```swift
// Show (make key, order front, persist enabled state)
func show() {
    isEnabled = true
    window?.makeKeyAndOrderFront(nil)
}

// Hide (order out, persist disabled state)
func hide() {
    isEnabled = false
    window?.orderOut(nil)
}

// Show temporarily (e.g., for browser tool activation)
func showTemporarily() {
    guard window != nil else { return }
    window?.normalizeForTemporaryShow()
    window?.makeKeyAndOrderFront(nil)
    // Does NOT persist enabled state
}
```

### 9.3 AI Conversation Lifecycle

```swift
// Open AI input:
func showAIConversation() {
    // Save center before expanding so we can restore position on close
    preChatCenter = NSPoint(x: frame.midX, y: frame.midY)

    // Resize from compact (40×14) to input panel (430×120)
    let inputSize = NSSize(width: FloatingControlBarWindow.expandedWidth, height: 120)
    resizeAnchored(to: inputSize, makeResizable: false, animated: true, anchorTop: true)

    withAnimation(.spring(response: 0.4, dampingFraction: 0.8)) {
        state.showingAIConversation = true
        state.showingAIResponse = false
    }

    makeKeyAndOrderFront(nil)
    focusInputField()
}

// Close AI conversation:
func closeAIConversation() {
    // Cancel any in-flight chat
    FloatingControlBarManager.shared.cancelChat()

    // Determine where to restore (saved center or default position)
    let size = FloatingControlBarWindow.minBarSize
    let restoreOrigin: NSPoint
    if !ShortcutSettings.shared.draggableBarEnabled {
        restoreOrigin = defaultPillOrigin()
    } else if let center = preChatCenter {
        restoreOrigin = NSPoint(x: center.x - size.width / 2, y: center.y - size.height / 2)
    } else {
        restoreOrigin = NSPoint(x: frame.midX - size.width / 2, y: frame.midY - size.height / 2)
    }

    // Animate collapse
    NSAnimationContext.beginGrouping()
    NSAnimationContext.current.duration = 0.3
    self.setFrame(NSRect(origin: restoreOrigin, size: size), display: true, animate: true)
    NSAnimationContext.endGrouping()

    withAnimation(.spring(response: 0.4, dampingFraction: 0.8)) {
        state.showingAIConversation = false
        state.showingAIResponse = false
    }

    preChatCenter = nil
}
```

---

## 10. Linux Port Considerations (Wayland & X11)

Porting the floating bar to Linux requires addressing fundamental architectural differences between macOS window management and Wayland/X11.

### 10.1 Window Type & Layering

| macOS | X11 | Wayland | Issue |
|-------|-----|---------|-------|
| NSPanel (non-activating) | `_NET_WM_WINDOW_TYPE_UTILITY` | `xdg_toplevel_layer_shell` | Wayland doesn't allow persistent "always-on-top"; requires special surface role |
| Window level `.floating` | `_NET_WM_STATE_ABOVE` | Compositor dependent (KDE supports `layer-shell`, GNOME doesn't) | No standard; compositor plugin may be needed |
| Ignores activation | `_NET_WM_WINDOW_TYPE_DOCK` + `_NET_WM_STATE_STICKY` | Not standardized | X11 has hints; Wayland relies on compositor policy |

**Linux Implementation:**

- **X11**: Use `_NET_WM_WINDOW_TYPE_UTILITY`, `_NET_WM_STATE_ABOVE`, `_NET_WM_WINDOW_OPACITY`
- **Wayland (GNOME)**: Fall back to regular window; compositor brings it to front on hotkey (no persistent layer)
- **Wayland (KDE)**: Use `wlr-layer-shell` if available; fall back to overlay surface
- **Fallback**: Regular window with always-on-top hint; accept that Wayland/fullscreen apps may cover it

### 10.2 Global Hotkeys

| macOS | X11 | Wayland | Issue |
|-------|-----|---------|-------|
| Carbon `RegisterEventHotKey` | `XGrabKey()` per root window | DBus keybindings (GNOME) or no standard | X11 is blocking; Wayland requires desktop-level integration |
| Truly global (all apps) | Yes, via root grab | No—other apps can override; requires WM cooperation | Wayland: Compositor controls all hotkeys; apps request via DBus |

**Linux Implementation:**

- **X11 (working, but inferior UX)**:
  ```c
  XGrabKey(dpy, keyCode, modifiers, root, true, GrabModeAsync, GrabModeAsync);
  ```
  Problem: If another app grabs the same key, conflict; no graceful fallback.

- **Wayland (GNOME 42+)**:
  ```rust
  // Use org.gnome.Shell keybindings DBus interface
  // User must register shortcut in GNOME Settings → Keyboard → Custom Shortcuts
  // App listens on DBus: org.gnome.Shell signal 'Accelerator'
  ```
  Problem: User must manually register shortcut; app can't do it autonomously.

- **Wayland (KDE)**:
  ```cpp
  // Use KGlobalShortcuts DBus interface
  // KDE manages all global shortcuts, app registers component
  ```

- **Fallback**: Detect Wayland compositor via `$WAYLAND_DISPLAY`; if set, disable global hotkeys and show UI hint ("Hotkeys unavailable on Wayland; use the app window").

### 10.3 Multi-Screen Tracking

| macOS | X11 | Wayland | Issue |
|-------|-----|---------|-------|
| `NSScreen.screens` list | `XRRGetScreenResourcesCurrent()` | Not directly available | Wayland: Compositor determines outputs; no portable API |
| `NSEvent.mouseLocation` (global) | `XQueryPointer()` | Not available; use `/sys/class/input/` or compositor DBus | Wayland: Privacy restriction; apps can't query absolute cursor |
| 250ms polling | Same; 250ms OK | Same; but no real API | X11/Wayland both need polling (no better option) |

**Linux Implementation:**

- **X11**: Use `XRRGetScreenResourcesCurrent()` to list monitors; `XQueryPointer()` for cursor; same 250ms interval.
- **Wayland**: No cursor API; use compositor DBus (GNOME: `org.gnome.Shell.Introspect.GetFocusedWindow()`); update window position only on explicit app activation or pointer events.

### 10.4 Drag & Window Movement

| macOS | X11 | Wayland | Issue |
|-------|-----|---------|-------|
| `setFrameOrigin()` (immediate) | `XMoveWindow()` (immediate) | `xdg_toplevel.move()` (asynchronous) | Wayland: App requests move; compositor decides |
| Animated resize | `XConfigureWindow()` + timer loop | Not supported; request → wait for compositor response | Wayland: Resize is async; window may lag behind cursor |

**Linux Implementation:**

- **X11**: Use `XMoveWindow()` immediately; drag feels responsive.
- **Wayland**: Call `xdg_toplevel.move(seat, serial)` on mouse-down; compositor handles drag; don't try to animate. Window may lag visually during drag.

### 10.5 Position & Size Persistence

| macOS | X11 | Wayland | Issue |
|-------|-----|---------|-------|
| `UserDefaults` (persistent across boots) | `$XDG_CONFIG_HOME/app.conf` | Same; but WM may override size on startup | Some WMs (GNOME) tile windows on startup |

**Linux Implementation:**

- Save to `~/.config/omi-floating-bar.json`: `{ "position": { "x": ..., "y": ... }, "size": { "width": ..., "height": ... } }`
- On startup, set window geometry; accept that Wayland compositors may override.
- Detect screen disconnects via `XRRScreenChangeNotifyEvent` (X11) or compositor DBus (Wayland); revalidate position.

### 10.6 Window Activation & Keyboard Focus

| macOS | X11 | Wayland | Issue |
|-------|-----|---------|-------|
| Non-activating panel; focus stays on other app | `_NET_WM_STATE_SKIP_TASKBAR` + `_NET_WM_WINDOW_TYPE_UTILITY` | `xdg_toplevel.set_modal()` or layered surface | Wayland: Hard to keep focus away from PTT input |

**Linux Implementation:**

- **X11**: Set window type `_NET_WM_WINDOW_TYPE_UTILITY`; activate keyboard focus only when needed (AI input).
- **Wayland**: Use `set_modal(false)` on toplevel; focus still taken on click (unavoidable); design UX to accept this.

### 10.7 Summary: Wayland Gotchas

| Feature | macOS | Wayland | Workaround |
|---------|-------|---------|-----------|
| Always-on-top | Native (NSPanel `.floating`) | Compositor-dependent; no standard API | Fall back to overlay surface or "bringToFront on timer" |
| Global hotkeys | Carbon APIs, reliable | Not available; must use compositor plugin | Disable hotkeys on Wayland; show UI hint |
| Cursor tracking | `NSEvent.mouseLocation` (free) | Not available (privacy) | Use explicit pointer events only |
| Drag to move | `setFrameOrigin()` (immediate) | `xdg_toplevel.move()` (async; laggy) | Accept lag; no workaround |
| Focus control | Non-activating panels available | Compositor takes focus on click (unavoidable) | Design for this; alt-tab focus model |
| Resize animations | `NSAnimationContext` + immediate apply | Compositor async; no animation support | Static resize, no animation |

### 10.8 Fallback Strategy for Wayland

```rust
// Tauri app.rs / main.rs
use std::env;

fn is_wayland() -> bool {
    env::var("WAYLAND_DISPLAY").is_ok() || env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
}

fn setup_floating_bar(app: &App, config: &Config) {
    if is_wayland() {
        // Wayland: Use overlay window or fallback to regular window
        // Disable global hotkeys; show in-app hint
        eprintln!("Floating bar: Wayland detected; global hotkeys disabled");
        eprintln!("Hint: Hotkeys unavailable on Wayland. Use the app window or GNOME Shell extension.");
        
        // Create window without layer-shell (or with layer-shell if KDE detected)
        let window = app.create_window(
            "floating_bar",
            url,
            WindowConfig {
                // Regular window; no always-on-top guarantee
                ..config.clone()
            },
        );
    } else {
        // X11: Full floating bar with global hotkeys
        setup_floating_bar_x11(app, config);
    }
}
```

---

## Summary

The Omi floating control bar is a **sophisticated macOS UI component** combining:

1. **Non-activating NSPanel** for seamless keyboard capture without disrupting other windows
2. **Carbon Event APIs** for truly global hotkey registration (Ask Omi shortcut)
3. **NSEvent monitors** for push-to-talk voice recording with double-tap locking
4. **SwiftUI + NSHostingView** with careful constraint management to avoid layout crashes
5. **250ms cursor polling** for multi-screen tracking and auto-following
6. **UserDefaults** persistence for position, size, and enabled state
7. **Isolated ChatProvider** for floating bar queries (separate from main history)

**Linux porting challenges:**

- **Wayland** doesn't support persistent always-on-top; requires compositor plugin or fallback
- **Global hotkeys** unavailable on Wayland without desktop integration
- **Cursor tracking** blocked by Wayland privacy model; use explicit pointer events
- **Drag & move** is async on Wayland; UX will feel laggy vs. macOS
- **Window focus** can't be avoided on Wayland; accept this limitation

Consider building a **Tauri + GTK4** app on Linux with:
- X11 support via `libx11` (XGrabKey, XMoveWindow, XQueryPointer)
- Wayland fallback: overlay surface + DBus keybinding hints + pointer-based tracking
- Graceful degradation: hotkeys → hint; always-on-top → overlay; drag → async request
