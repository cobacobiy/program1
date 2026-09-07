export interface HealthResponse {
  status: string;
  version: string;
  subsystems?: Record<string, string>;
}

export interface Product {
  id: string;
  name: string;
  description: string;
  price_cents: number;
  stock: number;
  image_url?: string | null;
  category?: string | null;
}

export interface CatalogPageResponse {
  items: Product[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface CartItem {
  product: Product;
  quantity: number;
}
