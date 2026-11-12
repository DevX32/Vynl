package com.vynl.vynl_mobile.ui.screens

import android.Manifest
import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.ExperimentalGetImage
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.KeyboardArrowDown
import androidx.compose.material.icons.rounded.NoPhotography
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import com.vynl.vynl_mobile.ui.AppViewModel
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.common.InputImage
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.guava.await
import androidx.compose.ui.text.font.FontWeight
import kotlinx.coroutines.launch
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine

data class PairingPayload(
    val host: String,
    val pin: String,
    val alts: List<String> = emptyList(),
)

fun parsePairingPayload(raw: String): PairingPayload? {
    val uri = runCatching { Uri.parse(raw.trim()) }.getOrNull() ?: return null
    if (!"vynl".equals(uri.scheme, ignoreCase = true)) return null
    if (!"pair".equals(uri.host, ignoreCase = true)) return null

    val host = uri.getQueryParameter("h")?.trim().orEmpty()
    val pin = uri.getQueryParameter("pin")?.trim().orEmpty()
    if (host.isEmpty() || pin.isEmpty()) return null

    val port = uri.getQueryParameter("p")?.trim().orEmpty()

    fun withPort(value: String): String {
        val cleaned = value.trim().removePrefix("http://").removePrefix("https://").removeSuffix("/")
        if (cleaned.isEmpty()) return cleaned
        if (Regex(""":\d+$""").containsMatchIn(cleaned)) return cleaned
        return if (port.isEmpty()) cleaned else "$cleaned:$port"
    }

    val alts = (uri.getQueryParameter("a") ?: "")
        .split(',')
        .map { it.trim() }
        .filter { it.isNotEmpty() }
        .map(::withPort)

    return PairingPayload(host = withPort(host), pin = pin, alts = alts)
}

@OptIn(ExperimentalGetImage::class)
@Composable
fun PairScreen(
    viewModel: AppViewModel,
    onClose: () -> Unit,
    onPaired: () -> Unit,
) {
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var viewfinderReady by remember { mutableStateOf(false) }
    var cameraGranted by remember { mutableStateOf(false) }
    var permissionRequested by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current

    BackHandler { onClose() }

    val requestPermission = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        cameraGranted = granted
        permissionRequested = true
        if (!granted) error = "Vynl needs the camera to read the pairing code."
    }

    LaunchedEffect(Unit) {
        if (!cameraGranted && !permissionRequested) {
            requestPermission.launch(Manifest.permission.CAMERA)
        }
    }

    val cameraProviderFuture = remember { ProcessCameraProvider.getInstance(context) }
    val preview = remember {
        Preview.Builder()
            .setResolutionSelector(
                ResolutionSelector.Builder()
                    .build(),
            )
            .build()
    }
    val analyzer = remember {
        ImageAnalysis.Builder()
            .setTargetResolution(android.util.Size(1280, 720))
            .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
            .build()
    }
    val executor = remember { ContextCompat.getMainExecutor(context) }
    var scanned by remember { mutableStateOf(false) }

    val scanner = remember {
        com.google.mlkit.vision.barcode.BarcodeScanning.getClient(
            com.google.mlkit.vision.barcode.BarcodeScannerOptions.Builder()
                .setBarcodeFormats(com.google.mlkit.vision.barcode.common.Barcode.FORMAT_QR_CODE)
                .build(),
        )
    }

    LaunchedEffect(cameraGranted, cameraProviderFuture, preview, analyzer) {
        if (!cameraGranted) return@LaunchedEffect
        runCatching {
            val provider = cameraProviderFuture.await()
            provider.unbindAll()
            provider.bindToLifecycle(
                lifecycleOwner,
                CameraSelector.DEFAULT_BACK_CAMERA,
                preview,
                analyzer,
            )
            viewfinderReady = true
        }.onFailure {
            error = "Camera didn't open."
        }
    }

    DisposableEffect(analyzer) {
        analyzer.setAnalyzer(executor) { image ->
            if (!scanned) {
                val mediaImage = image.image
                if (mediaImage == null) {
                    image.close()
                    return@setAnalyzer
                }
                val inputImage = com.google.mlkit.vision.common.InputImage.fromMediaImage(
                    mediaImage,
                    image.imageInfo.rotationDegrees,
                )
                scanner.process(inputImage)
                    .addOnSuccessListener { barcodes ->
                        for (barcode in barcodes) {
                            val text = barcode.rawValue ?: continue
                            if (text.isBlank()) continue
                            val payload = parsePairingPayload(text)
                            if (payload == null) {
                                error = "That isn't a Vynl pairing code."
                                continue
                            }
                            if (scanned) return@addOnSuccessListener
                            scanned = true
                            if (!busy) {
                                busy = true
                                scope.launch(Dispatchers.IO) {
                                    val failure = viewModel.pairDesktop(
                                        address = payload.host,
                                        pin = payload.pin,
                                        alts = payload.alts,
                                        autoSync = true,
                                    )
                                    busy = false
                                    if (failure == null) onPaired() else error = failure
                                }
                            }
                            break
                        }
                        image.close()
                    }
                    .addOnFailureListener {
                        image.close()
                    }
            } else {
                image.close()
            }
        }
        onDispose {
            analyzer.clearAnalyzer()
            scanner.close()
        }
    }

    Box(
        modifier = Modifier.fillMaxSize().background(VynlColors.bg),
    ) {
        if (cameraGranted) {
            AndroidView(
                factory = { ctx ->
                    PreviewView(ctx).apply {
                        scaleType = PreviewView.ScaleType.FILL_CENTER
                        preview.setSurfaceProvider(this.surfaceProvider)
                    }
                },
                modifier = Modifier.fillMaxSize(),
            )
            if (!viewfinderReady && error == null && !busy) {
                CircularProgressIndicator(
                    color = VynlColors.accent,
                    strokeWidth = 2.2.dp,
                    modifier = Modifier
                        .align(Alignment.Center)
                        .size(32.dp),
                )
            }
        } else {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(horizontal = 34.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = androidx.compose.foundation.layout.Arrangement.Center,
            ) {
                Icon(
                    Icons.Rounded.NoPhotography,
                    contentDescription = null,
                    tint = VynlColors.faint,
                    modifier = Modifier.size(34.dp),
                )
                Spacer(Modifier.height(18.dp))
                Text(
                    text = "Camera access needed",
                    color = VynlColors.text,
                    fontSize = 19.sp,
                    fontWeight = FontWeight.W700,
                    letterSpacing = (-0.3).sp,
                    fontFamily = VynlFonts.display,
                )
                Spacer(Modifier.height(8.dp))
                Text(
                    text = error ?: "Vynl needs the camera to read the pairing code.",
                    color = VynlColors.dim,
                    fontSize = 13.5.sp,
                    lineHeight = 21.sp,
                    textAlign = TextAlign.Center,
                    fontFamily = VynlFonts.display,
                )
                Spacer(Modifier.height(20.dp))
                androidx.compose.material3.OutlinedButton(
                    onClick = {
                        error = null
                        permissionRequested = false
                        requestPermission.launch(Manifest.permission.CAMERA)
                    },
                    shape = RoundedCornerShape(VynlRadius.control),
                ) {
                    Text(
                        text = "Try again",
                        color = VynlColors.text,
                        fontFamily = VynlFonts.display,
                    )
                }
            }
        }

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .statusBarsPadding()
                .padding(horizontal = 16.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onClose) {
                Icon(
                    Icons.Rounded.KeyboardArrowDown,
                    contentDescription = "Back to settings",
                    tint = VynlColors.dim,
                    modifier = Modifier.size(30.dp),
                )
            }
            Text(
                text = "Scan QR",
                modifier = Modifier.weight(1f),
                textAlign = TextAlign.Center,
                color = VynlColors.text,
                fontSize = 16.sp,
                fontWeight = FontWeight.W700,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.width(48.dp))
        }

        Box(
            modifier = Modifier
                .fillMaxSize()
                .navigationBarsPadding()
                .padding(horizontal = 30.dp, vertical = 22.dp),
            contentAlignment = Alignment.BottomCenter,
        ) {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                when {
                    busy -> Text(
                        text = "Pairing…",
                        color = VynlColors.accent,
                        fontSize = 12.sp,
                        fontWeight = FontWeight.W700,
                        letterSpacing = 0.2.sp,
                        fontFamily = VynlFonts.display,
                    )
                    error != null -> Text(
                        text = error as String,
                        color = VynlColors.danger,
                        fontSize = 12.sp,
                        letterSpacing = 0.2.sp,
                        textAlign = TextAlign.Center,
                        fontFamily = VynlFonts.display,
                    )
                    else -> Text(
                        text = "Scanning for a code…",
                        color = VynlColors.faint,
                        fontSize = 12.sp,
                        letterSpacing = 0.2.sp,
                        fontFamily = VynlFonts.display,
                    )
                }
                Spacer(Modifier.height(8.dp))
                Text(
                    text = "Point at the QR shown in Vynl desktop Settings → Mobile Sync.",
                    color = VynlColors.dim,
                    fontSize = 12.5.sp,
                    lineHeight = 19.sp,
                    textAlign = TextAlign.Center,
                    fontFamily = VynlFonts.display,
                )
            }
        }
    }
}
