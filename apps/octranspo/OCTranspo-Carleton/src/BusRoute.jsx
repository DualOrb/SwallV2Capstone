import "./BusRoute.css"
import TripTime from "./TripTime.jsx"

export default function BusRoute( routeData ) {

    const styles = {
        "train-support": {
            "border-color": {
                "borderColor": routeData["routeData"]["RouteNo"] !== "2" ? "#E91C23" : "#24cc14",
            },
            "background-color": {
                "backgroundColor": routeData["routeData"]["RouteNo"] !== "2" ? "#E91C23" : "#24cc14",
            }
        }
    }

    return (
            <div className="route-container">
                <div className="bus-num" style={styles["train-support"]["background-color"]}>
                    <h3 >{routeData["routeData"]["RouteNo"]}</h3>
                </div>
                <div className="route-data">
                    <div className="bus-heading" style={styles["train-support"]["border-color"]}>
                        <h3>{routeData["routeData"]["RouteHeading"]}</h3>
                    </div>
                    <div className="trip-times-container">
                        <div className="trip-times">
                            {routeData["routeData"]["Trips"].map(function (tripData) {
                                return (
                                    <TripTime time={tripData}></TripTime> 
                                )
                            })}
                        </div>
                    </div>
                </div>
            </div>
    )
} 