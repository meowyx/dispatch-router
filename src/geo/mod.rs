use crate::models::courier::GeoPoint;

const EARTH_RADIUS_KM: f64 = 6_371.0;

pub fn haversine_km(a: &GeoPoint, b: &GeoPoint) -> f64 {
    let lat1 = a.lat.to_radians();
    let lat2 = b.lat.to_radians();
    let delta_lat = (b.lat - a.lat).to_radians();
    let delta_lng = (b.lng - a.lng).to_radians();

    let sin_lat = (delta_lat / 2.0).sin();
    let sin_lng = (delta_lng / 2.0).sin();

    let haversine = sin_lat * sin_lat + lat1.cos() * lat2.cos() * sin_lng * sin_lng;
    let central_angle = 2.0 * haversine.sqrt().asin();

    EARTH_RADIUS_KM * central_angle
}

#[cfg(test)]
mod tests {
    use super::haversine_km;
    use crate::models::courier::GeoPoint;

    #[test]
    fn zero_distance_for_same_point() {
        let p = GeoPoint {
            lat: 53.5511,
            lng: 9.9937,
        };
        let distance = haversine_km(&p, &p);
        assert!(distance < 1e-9);
    }

    #[test]
    fn london_to_paris_is_around_343_km() {
        let london = GeoPoint {
            lat: 51.5074,
            lng: -0.1278,
        };
        let paris = GeoPoint {
            lat: 48.8566,
            lng: 2.3522,
        };
        let distance = haversine_km(&london, &paris);
        assert!((distance - 343.0).abs() < 5.0);
    }
}
