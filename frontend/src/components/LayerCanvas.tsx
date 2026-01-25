import { useRef, useState, useCallback } from 'preact/hooks'
import { ComposerLayer } from '../api'

interface LayerCanvasProps {
  layers: ComposerLayer[]
  selectedIndex: number | null
  canvasWidth: number
  canvasHeight: number
  onSelect: (index: number | null) => void
  onUpdate: (index: number, updates: Partial<ComposerLayer>) => void
}

type DragState = {
  type: 'move' | 'resize'
  layerIndex: number
  corner?: 'nw' | 'ne' | 'sw' | 'se'
  startX: number
  startY: number
  startLayer: ComposerLayer
} | null

const HANDLE_SIZE = 12
const MIN_SIZE = 10

export function LayerCanvas({
  layers,
  selectedIndex,
  canvasWidth,
  canvasHeight,
  onSelect,
  onUpdate,
}: LayerCanvasProps) {
  const svgRef = useRef<SVGSVGElement>(null)
  const [dragState, setDragState] = useState<DragState>(null)
  const [hoverIndex, setHoverIndex] = useState<number | null>(null)
  const [isMouseOver, setIsMouseOver] = useState(false)

  // Convert mouse event to SVG coordinates
  const getSvgPoint = useCallback((e: MouseEvent): { x: number; y: number } | null => {
    const svg = svgRef.current
    if (!svg) return null

    const rect = svg.getBoundingClientRect()
    const scaleX = canvasWidth / rect.width
    const scaleY = canvasHeight / rect.height

    return {
      x: (e.clientX - rect.left) * scaleX,
      y: (e.clientY - rect.top) * scaleY,
    }
  }, [canvasWidth, canvasHeight])

  // Find topmost layer at point
  const findLayerAtPoint = useCallback((x: number, y: number): number | null => {
    // Iterate in reverse (topmost layer is last in array)
    for (let i = layers.length - 1; i >= 0; i--) {
      const layer = layers[i]
      if (
        x >= layer.x &&
        x <= layer.x + layer.width &&
        y >= layer.y &&
        y <= layer.y + layer.height
      ) {
        return i
      }
    }
    return null
  }, [layers])

  const handleMouseDown = useCallback((e: MouseEvent, layerIndex: number, corner?: 'nw' | 'ne' | 'sw' | 'se') => {
    e.preventDefault()
    e.stopPropagation()

    const point = getSvgPoint(e)
    if (!point) return

    const layer = layers[layerIndex]
    onSelect(layerIndex)

    setDragState({
      type: corner ? 'resize' : 'move',
      layerIndex,
      corner,
      startX: point.x,
      startY: point.y,
      startLayer: { ...layer },
    })
  }, [layers, getSvgPoint, onSelect])

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!dragState) return

    const point = getSvgPoint(e)
    if (!point) return

    const dx = point.x - dragState.startX
    const dy = point.y - dragState.startY
    const { startLayer, layerIndex, type, corner } = dragState

    if (type === 'move') {
      onUpdate(layerIndex, {
        x: Math.round(startLayer.x + dx),
        y: Math.round(startLayer.y + dy),
      })
    } else if (type === 'resize' && corner) {
      let newX = startLayer.x
      let newY = startLayer.y
      let newWidth = startLayer.width
      let newHeight = startLayer.height

      // Default = proportional scaling, Shift key = free resize
      if (!e.shiftKey) {
        const aspectRatio = startLayer.width / startLayer.height
        // Use the larger delta to determine scale
        const absDx = Math.abs(dx)
        const absDy = Math.abs(dy)

        let scaledDx = dx
        let scaledDy = dy

        if (absDx > absDy * aspectRatio) {
          // Width change dominates
          scaledDy = (absDx / aspectRatio) * Math.sign(dy || dx)
        } else {
          // Height change dominates
          scaledDx = (absDy * aspectRatio) * Math.sign(dx || dy)
        }

        // Apply proportional changes based on corner
        if (corner === 'se') {
          newWidth = startLayer.width + scaledDx
          newHeight = startLayer.height + scaledDy
        } else if (corner === 'sw') {
          newX = startLayer.x - scaledDx
          newWidth = startLayer.width + scaledDx
          newHeight = startLayer.height + scaledDy
        } else if (corner === 'ne') {
          newY = startLayer.y - scaledDy
          newWidth = startLayer.width + scaledDx
          newHeight = startLayer.height + scaledDy
        } else if (corner === 'nw') {
          newX = startLayer.x - scaledDx
          newY = startLayer.y - scaledDy
          newWidth = startLayer.width + scaledDx
          newHeight = startLayer.height + scaledDy
        }
      } else {
        // Normal resize - handle each corner independently
        if (corner.includes('w')) {
          newX = startLayer.x + dx
          newWidth = startLayer.width - dx
        }
        if (corner.includes('e')) {
          newWidth = startLayer.width + dx
        }
        if (corner.includes('n')) {
          newY = startLayer.y + dy
          newHeight = startLayer.height - dy
        }
        if (corner.includes('s')) {
          newHeight = startLayer.height + dy
        }
      }

      // Enforce minimum size
      if (newWidth < MIN_SIZE) {
        if (corner.includes('w')) {
          newX = startLayer.x + startLayer.width - MIN_SIZE
        }
        newWidth = MIN_SIZE
      }
      if (newHeight < MIN_SIZE) {
        if (corner.includes('n')) {
          newY = startLayer.y + startLayer.height - MIN_SIZE
        }
        newHeight = MIN_SIZE
      }

      onUpdate(layerIndex, {
        x: Math.round(newX),
        y: Math.round(newY),
        width: Math.round(newWidth),
        height: Math.round(newHeight),
      })
    }
  }, [dragState, getSvgPoint, onUpdate])

  const handleMouseUp = useCallback(() => {
    setDragState(null)
  }, [])

  const handleMouseEnter = useCallback(() => {
    setIsMouseOver(true)
  }, [])

  const handleMouseLeave = useCallback(() => {
    setIsMouseOver(false)
    setHoverIndex(null)
    if (!dragState) {
      // Don't clear drag state if we're actively dragging
    }
  }, [dragState])

  const handleBackgroundClick = useCallback((e: MouseEvent) => {
    const point = getSvgPoint(e)
    if (!point) return

    const layerIndex = findLayerAtPoint(point.x, point.y)
    if (layerIndex !== null) {
      onSelect(layerIndex)
    } else {
      onSelect(null)
    }
  }, [getSvgPoint, findLayerAtPoint, onSelect])

  const handleLayerMouseEnter = useCallback((index: number) => {
    if (!dragState) {
      setHoverIndex(index)
    }
  }, [dragState])

  const handleLayerMouseLeave = useCallback(() => {
    if (!dragState) {
      setHoverIndex(null)
    }
  }, [dragState])

  // Cursor based on corner
  const getHandleCursor = (corner: string): string => {
    switch (corner) {
      case 'nw': case 'se': return 'nwse-resize'
      case 'ne': case 'sw': return 'nesw-resize'
      default: return 'pointer'
    }
  }

  // Render handles for selected layer
  const renderHandles = (layer: ComposerLayer, index: number) => {
    const corners: Array<{ id: 'nw' | 'ne' | 'sw' | 'se'; cx: number; cy: number }> = [
      { id: 'nw', cx: layer.x, cy: layer.y },
      { id: 'ne', cx: layer.x + layer.width, cy: layer.y },
      { id: 'sw', cx: layer.x, cy: layer.y + layer.height },
      { id: 'se', cx: layer.x + layer.width, cy: layer.y + layer.height },
    ]

    // Scale handle size based on viewBox
    const handleRadius = (HANDLE_SIZE / 2) * (canvasWidth / 576)

    return corners.map(({ id, cx, cy }) => (
      <circle
        key={id}
        class="resize-handle"
        cx={cx}
        cy={cy}
        r={handleRadius}
        style={{ cursor: getHandleCursor(id) }}
        onMouseDown={(e) => handleMouseDown(e as unknown as MouseEvent, index, id)}
      />
    ))
  }

  if (canvasHeight <= 0) return null

  // Show overlay only when mouse is over the preview (or actively dragging)
  const showOverlay = isMouseOver || dragState !== null

  return (
    <svg
      ref={svgRef}
      class={`layer-overlay ${showOverlay ? 'visible' : 'hidden'}`}
      viewBox={`0 0 ${canvasWidth} ${canvasHeight}`}
      preserveAspectRatio="xMidYMid meet"
      onMouseMove={handleMouseMove as unknown as (e: Event) => void}
      onMouseUp={handleMouseUp}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={() => {
        handleMouseLeave()
        handleMouseUp()
      }}
      onClick={handleBackgroundClick as unknown as (e: Event) => void}
    >
      {/* Layer rectangles - only render when overlay is visible */}
      {showOverlay && layers.map((layer, index) => {
        const isSelected = selectedIndex === index
        const isHovered = hoverIndex === index

        return (
          <g key={index}>
            <rect
              class={`layer-box ${isSelected ? 'selected' : ''} ${isHovered ? 'hovered' : ''}`}
              x={layer.x}
              y={layer.y}
              width={layer.width}
              height={layer.height}
              onMouseDown={(e) => handleMouseDown(e as unknown as MouseEvent, index)}
              onMouseEnter={() => handleLayerMouseEnter(index)}
              onMouseLeave={handleLayerMouseLeave}
            />
            {isSelected && renderHandles(layer, index)}
          </g>
        )
      })}
    </svg>
  )
}
