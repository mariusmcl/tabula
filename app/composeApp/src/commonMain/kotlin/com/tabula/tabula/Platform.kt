package com.tabula.tabula

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform