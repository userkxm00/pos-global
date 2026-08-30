// TypeScript types matching Rust DTOs for F2.05 Variants / Matrix.

export interface AttributeDefinition {
  id: string
  name: string
  sort_order: number
  created_at: string
}

export interface AttributeValue {
  id: string
  attribute_definition_id: string
  value: string
  sort_order: number
  created_at: string
}

export interface ProductVariant {
  id: string
  product_id: string
  sku: string | null
  barcode: string | null
  price_override_minor: number | null
  cost_price_minor: number | null
  is_active: boolean
  created_at: string
  updated_at: string
  deleted_at: string | null
}

export interface VariantWithAttributes {
  variant: ProductVariant
  attribute_values: AttributeValue[]
}

export interface CreateAttributeDefinitionInput {
  name: string
  sort_order?: number | null
}

export interface CreateAttributeValueInput {
  attribute_definition_id: string
  value: string
  sort_order?: number | null
}

export interface CreateVariantInput {
  product_id: string
  sku?: string | null
  barcode?: string | null
  price_override_minor?: number | null
  cost_price_minor?: number | null
  attribute_value_ids: string[]
}

export interface UpdateVariantInput {
  id: string
  sku?: string | null
  barcode?: string | null
  price_override_minor?: number | null
  cost_price_minor?: number | null
  is_active: boolean
}

export interface MatrixDimensionInput {
  attribute_definition_id: string
  attribute_value_ids: string[]
}

export interface GenerateMatrixInput {
  product_id: string
  dimensions: MatrixDimensionInput[]
  default_price_override_minor?: number | null
  default_cost_price_minor?: number | null
  sku_prefix?: string | null
}

export interface PreviewMatrixInput {
  product_id: string
  dimensions: MatrixDimensionInput[]
}

export interface MatrixCombinationPreview {
  attribute_values: AttributeValue[]
  existing_variant_id: string | null
  is_new: boolean
}

export interface MatrixPreviewResult {
  total_combinations: number
  new_combinations_count: number
  existing_combinations_count: number
  combinations: MatrixCombinationPreview[]
}

export interface MatrixGenerationResult {
  total_combinations: number
  created_count: number
  existing_count: number
  created_variants: VariantWithAttributes[]
  existing_variants: VariantWithAttributes[]
}

export interface BulkUpdateVariantStatusInput {
  variant_ids: string[]
  is_active: boolean
}

export interface BulkUpdateVariantPricesInput {
  variant_ids: string[]
  price_override_minor?: number | null
  cost_price_minor?: number | null
}

export interface BulkOperationResult {
  updated_count: number
  affected_variant_ids: string[]
}
