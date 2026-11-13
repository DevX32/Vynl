package com.vynl.vynl_mobile.data

import kotlin.random.Random

private val adjectives = listOf(
    "Cosmic", "Velvet", "Neon", "Quiet", "Golden", "Midnight", "Paper", "Wild",
    "Silver", "Lunar", "Electric", "Soft", "Amber", "Static", "Marble", "Feral",
    "Polar", "Analog", "Distant", "Tidal", "Crimson", "Hollow", "Mellow",
    "Restless", "Sable", "Bright", "Dusty", "Endless", "Faint", "Frozen",
    "Gentle", "Heavy", "Hidden", "Iron", "Lonesome", "Muddy", "Nimble", "Opal",
    "Patient", "Plain", "Quietly", "Rough", "Sleepy", "Steady", "Stony",
    "Sunken", "Teal", "Thin", "Wandering", "Woven", "Arctic", "Blurred",
    "Copper", "Drowsy", "Eager", "Faintly", "Gilded", "Hazy", "Idle", "Jaded",
    "Lucid", "Mossy", "Narrow", "Pearly", "Quietest", "Ragged", "Sandy",
    "Silent", "Tender", "Vacant", "Wintry",
)

private val nouns = listOf(
    "Panda", "Echo", "Harbor", "Comet", "Meadow", "Signal", "Lantern", "Orchid",
    "Cinder", "Drift", "Willow", "Ember", "Atlas", "Fathom", "Prism", "Nomad",
    "Canyon", "Verge", "Halo", "Badger", "Beacon", "Cedar", "Cove", "Dahlia",
    "Delta", "Dune", "Estuary", "Fern", "Fjord", "Foundry", "Grove", "Harbour",
    "Hollow", "Isle", "Jetty", "Kite", "Lagoon", "Lark", "Lichen", "Mantis",
    "Meridian", "Monsoon", "Moss", "Nettle", "Onyx", "Owl", "Pebble", "Petrel",
    "Quarry", "Reef", "Ridge", "Rill", "Sparrow", "Summit", "Thicket",
    "Thistle", "Tundra", "Vesper", "Vista", "Wavelength", "Whistler",
    "Willowby", "Yarrow", "Zephyr", "Basil", "Bramble", "Clover", "Fennel",
    "Hare", "Heron", "Ibis", "Juniper", "Kestrel", "Moth", "Osprey", "Otter",
    "Raven", "Rowan", "Sorrel", "Vireo", "Wombat", "Wren",
)

fun randomNickname(): String =
    "${adjectives[Random.nextInt(adjectives.size)]} ${nouns[Random.nextInt(nouns.size)]}"

fun resolveDisplayName(configured: String, nickname: String): String {
    val name = configured.trim()
    return if (name.isEmpty()) nickname else name
}
